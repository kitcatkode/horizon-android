//! The open inventory, container, station or creative screen on the panel instead of the main
//! screen. It is laid out and hit-tested exactly as on the main window (its GUI pixels, its
//! cursor), then moved above the panel's tabs, at the largest GUI scale it fits. A
//! touch that starts on it becomes the main window's pointer: moves and a left button pressed
//! and released, mapped back to the main window.
use super::{
    super::{Game, Screen, event_loop::EventLoopCtl},
    tabs,
};
use crate::second_screen::{Phase, Touch};
use horizon_client::{creative::Creative, hud::HudBatch};
use std::cell::Cell;
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    window::WindowId,
};

/// What the last panel frame drew, for the touches before the next.
#[derive(Clone, Copy, Debug)]
pub(super) struct Drawn {
    pub(super) scale: u32,
    /// The panel's size in pixels.
    pub(super) size: [u32; 2],
    /// The hotbar's own GUI scale.
    pub(super) hotbar_scale: u32,
    /// The tabs' bottom edge (the hotbar is under it), in the panel's GUI pixels.
    pub(super) bottom: f32,
    /// Where the hosted screen went, if one was drawn.
    pub(super) offset: Option<Offset>,
}

/// The hosted screen's place: panel GUI = main GUI + `at`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Offset {
    at: [f32; 2],
    /// The main window's GUI scale, for the pointer's window pixels.
    main_scale: f32,
}

/// How the panel lays out the hosted screen this frame.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    pub(super) scale: u32,
    /// The main window's size, in pixels.
    main: [u32; 2],
    /// The screen's extent on the main window, in its GUI pixels: x, y, width, height.
    bounds: [f32; 4],
}

thread_local! {
    static DRAWN: Cell<Option<Drawn>> = const { Cell::new(None) };
    /// A touch began on the hosted screen: its moves and release go there too.
    static POINTER: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn drawn() -> Option<Drawn> {
    DRAWN.get()
}

pub(super) fn set_drawn(drawn: Option<Drawn>) {
    DRAWN.set(drawn);
}

/// The vanilla creative inventory's tabs stick out 28 pixels above and below it.
const CREATIVE_TABS: f32 = 28.0;

/// A touch at `point` (panel GUI pixels) for the hosted screen: false when it isn't one (it
/// began elsewhere on the panel, or no screen is hosted).
pub(super) fn screen_touch(
    game: &mut Game<'_>,
    drawn: &Drawn,
    touch: &Touch,
    point: [f32; 2],
    events: &dyn EventLoopCtl,
    id: WindowId,
) -> bool {
    if touch.phase == Phase::Started {
        let on_screen = drawn.offset.is_some()
            && game.screen_hosted()
            && point[1] < drawn.bottom
            && !tabs::on_tabs(point);
        POINTER.set(on_screen);
        if !on_screen {
            return false;
        }
    } else if !POINTER.get() {
        return false;
    }
    if matches!(touch.phase, Phase::Ended | Phase::Cancelled) {
        POINTER.set(false);
    }
    // The screen closed under the finger (`reset_inventory_clicks` dropped its press).
    let Some(offset) = drawn.offset.filter(|_| game.screen_hosted()) else {
        POINTER.set(false);
        return true;
    };
    let main = [point[0] - offset.at[0], point[1] - offset.at[1]];
    let device_id = DeviceId::dummy();
    game.handle_window_event(
        events,
        id,
        WindowEvent::CursorMoved {
            device_id,
            position: PhysicalPosition::new(
                f64::from(main[0] * offset.main_scale),
                f64::from(main[1] * offset.main_scale),
            ),
        },
    );
    if touch.phase != Phase::Moved {
        game.handle_window_event(
            events,
            id,
            WindowEvent::MouseInput {
                device_id,
                button: MouseButton::Left,
                state: if touch.phase == Phase::Started {
                    ElementState::Pressed
                } else {
                    ElementState::Released
                },
            },
        );
    }
    // A text field (a creative search, an anvil's name) takes the keyboard, as on the main
    // screen.
    if touch.phase == Phase::Ended && crate::platform::field_at(main) {
        game.android_show_keyboard();
    }
    true
}

/// The screen's backdrop dims the whole main window; on the panel the bars and tabs around it
/// stay as they are, so it is left out.
fn hide_backdrop(screen: &mut HudBatch) {
    let (w, h) = (screen.width, screen.height);
    for quad in screen.vertices.chunks_exact_mut(6) {
        let full = quad
            .iter()
            .all(|v| (v.pos[0] <= 0.0 || v.pos[0] >= w) && (v.pos[1] <= 0.0 || v.pos[1] >= h));
        if full {
            for v in quad {
                v.color = 0;
            }
        }
    }
}

impl Game<'_> {
    /// The open screen shows on the panel instead of the main screen: the inventory with every
    /// container, station and mount menu (`Screen::Inventory`), and the creative inventory.
    pub(in crate::game) fn screen_hosted(&self) -> bool {
        self.hud_on_second_screen()
            && self.title.is_none()
            && matches!(self.screen, Screen::Inventory | Screen::Creative)
            && self.server_packs.prompt.is_none()
    }

    /// The open screen's extent on the main window `main`: its image, the creative tabs, a
    /// recipe book beside it, the wider beacon and merchant screens.
    fn hosted_bounds(&self, main: &HudBatch) -> [f32; 4] {
        if self.screen == Screen::Creative {
            let (x, y) = Self::creative_origin(main);
            return [
                x,
                y - CREATIVE_TABS,
                Creative::WIDTH,
                Creative::HEIGHT + 2.0 * CREATIVE_TABS,
            ];
        }
        let (x, y) = self.inventory_origin(main);
        let (mut left, mut width) = (x, self.native_screen_width().unwrap_or(176.0));
        match self.station.as_ref().map(|s| s.kind) {
            Some(super::super::stations::Kind::Beacon) => {
                left += super::super::beacon::SHIFT;
                width -= 2.0 * super::super::beacon::SHIFT;
            }
            Some(super::super::stations::Kind::Merchant) => {
                left += super::super::trading::SHIFT;
                width -= 2.0 * super::super::trading::SHIFT;
            }
            _ => {}
        }
        let right = left + width;
        if let Some(book) = self.recipe_book_left(main) {
            left = left.min(book);
        }
        [left, y, right - left, self.screen_height()]
    }

    /// The hosted screen's layout on a panel `size` pixels big with the hotbar from
    /// `hotbar_top` (pixels) down: the largest scale (up to the panel's own) at which it fits
    /// above the tabs.
    pub(super) fn hosted_layout(&self, size: [u32; 2], hotbar_top: f32) -> Option<Layout> {
        let renderer = self.renderer.as_ref()?;
        let main = [renderer.config.width, renderer.config.height];
        let bounds = self.hosted_bounds(&HudBatch::new(main));
        let mut scale = super::gui_scale(size);
        while scale > 1 {
            let width = size[0] as f32 / scale as f32;
            let room = super::tabs_bottom(hotbar_top, scale) - tabs::TABS_HEIGHT - super::TOP;
            if bounds[2] + 4.0 <= width && bounds[3] <= room {
                break;
            }
            scale -= 1;
        }
        Some(Layout {
            scale,
            main,
            bounds,
        })
    }

    /// Draw the open screen as the main window would, with the pad's pointer, and move it
    /// between `top` and the tabs (ending at `bottom`).
    pub(super) fn draw_hosted(
        &mut self,
        batch: &mut HudBatch,
        layout: Layout,
        top: f32,
        bottom: f32,
    ) -> Offset {
        let mut screen = HudBatch::new(layout.main);
        screen.game_time = batch.game_time;
        if self.screen == Screen::Creative {
            self.draw_creative(&mut screen);
        } else {
            self.draw_inventory(&mut screen);
            self.draw_mod_browser(&mut screen);
        }
        self.draw_pad_cursor(&mut screen);
        hide_backdrop(&mut screen);
        let [x, y, w, h] = tabs::screen_area(batch.width, top, bottom);
        let [bx, by, bw, bh] = layout.bounds;
        let at = [
            (x + (w - bw) / 2.0).floor() - bx,
            (y + ((h - bh) / 2.0).max(0.0)).floor() - by,
        ];
        let main_scale = screen.scale;
        batch.append(screen, at[0], at[1]);
        Offset { at, main_scale }
    }
}

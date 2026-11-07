//! The second screen's panel (`crate::second_screen`: the AYN Thor's lower screen). The hotbar
//! moves there, enlarged along its top edge, over the selected tab's content and the
//! vanilla-styled tabs along the bottom (`tabs`: map, inventory, chat); the top screen keeps the world and what is drawn over it (crosshair,
//! the bars where the hotbar was, effects, titles, chat, every other screen). A tap on a hotbar
//! slot selects it. The inventory and every container screen open on the panel instead
//! (`host`), touched directly. Without a second screen nothing changes.
use super::{Game, event_loop::EventLoopCtl};
use crate::second_screen::{self as bridge, FAILED, Phase, SHOWN, Shown, Touch};
use horizon_client::{
    gfx::Renderer,
    hud::{HudAtlas, HudBatch},
    osk::Output,
};
use std::time::{Duration, Instant};
use winit::window::WindowId;

mod chat;
mod host;
mod keyboard;
mod tabs;
pub(super) use tabs::map_blocks_per_pixel;

/// The panel redraws at most this often: it shows status that changes slowly next to the world.
const INTERVAL: Duration = Duration::from_millis(16);
/// The panel's GUI keeps at least this many GUI pixels, so the hotbar is large enough to tap.
const MIN_GUI: [u32; 2] = [300, 200];

/// The hotbar's GUI pixels across the panel: the hotbar (182) with room for the off hand slot
/// or the attack indicator (29) on either side.
const HOTBAR_ROOM: u32 = 182 + 2 * 29;
/// The tabs' distance from the bottom edge.
const EDGE: f32 = 2.0;
/// The hotbar's distance from the top edge (its hotbar GUI pixels).
const HOTBAR_MARGIN: f32 = 1.0;

/// The hotbar's own GUI scale: the largest that fits `HOTBAR_ROOM` across the panel.
fn hotbar_scale(size: [u32; 2]) -> u32 {
    (size[0] / HOTBAR_ROOM).max(1)
}

/// How far the hotbar moves from where vanilla lays it out (its selected slot's frame, 23 high,
/// at the bottom) to the top edge, in GUI pixels at the hotbar's `scale`: negative, upwards.
fn hotbar_shift(size: [u32; 2], scale: u32) -> f32 {
    HOTBAR_MARGIN - ((size[1] as f32 / scale as f32).ceil() - 23.0)
}

/// Where the tabs' content starts, one GUI pixel under the hotbar, at the panel's GUI `scale`.
fn content_top(hotbar_scale: u32, scale: u32) -> f32 {
    ((HOTBAR_MARGIN + 23.0) * hotbar_scale as f32 / scale as f32).ceil() + 1.0
}

/// The tabs' bottom edge at the panel's GUI `scale`.
fn tabs_bottom(size: [u32; 2], scale: u32) -> f32 {
    (size[1] as f32 / scale as f32).floor() - EDGE
}

/// The selected tab's panel (GUI pixels) and the panel's GUI scale, on a panel `size` pixels
/// big showing no screen of the game's.
fn tab_panel(size: [u32; 2]) -> ([f32; 4], u32) {
    let scale = gui_scale(size);
    let top = content_top(hotbar_scale(size), scale);
    let width = (size[0] as f32 / scale as f32).ceil();
    (
        tabs::panel_rect(width, top, tabs_bottom(size, scale)),
        scale,
    )
}

/// The chat screen's box on the panel (its GUI width and height), while one is shown.
pub(super) fn chat_box() -> Option<[f32; 2]> {
    let size = SHOWN.with_borrow(|s| s.as_ref().map(|s| s.panel.size()))?;
    Some(chat::chat_box(size))
}

/// The largest GUI scale that keeps `MIN_GUI`.
fn gui_scale(size: [u32; 2]) -> u32 {
    let mut scale = 1;
    while size[0] / (scale + 1) >= MIN_GUI[0] && size[1] / (scale + 1) >= MIN_GUI[1] {
        scale += 1;
    }
    scale
}

impl Game<'_> {
    /// The panel shows the hotbar and the map: the main screen leaves them out.
    pub(super) fn hud_on_second_screen(&self) -> bool {
        !self.options.split_guest && self.player.is_some() && SHOWN.with_borrow(Option::is_some)
    }

    /// Once a frame, after the main screen's: follow the second screen's Surface, apply its
    /// touches (a screen shown there takes them as the main window's pointer: `events`, `id`)
    /// and redraw the panel.
    pub(super) fn second_screen_frame(&mut self, events: &dyn EventLoopCtl, id: WindowId) {
        if self.options.split_guest {
            return;
        }
        let Some(surface) = follow_surface(self.renderer.as_ref()) else {
            return;
        };
        // No world (the title menus after one closed): the menus' keyboard.
        if self.player.is_none() || self.title.is_some() {
            let field = self.title.as_mut().and_then(|t| {
                let typing = t.has_caret_field();
                let text = t.focused_field().map(|(_, s, _)| s.clone());
                typing.then(|| text.unwrap_or_default())
            });
            let Some(renderer) = self.renderer.as_ref() else {
                return;
            };
            let typed = menu_panel(renderer, &self.gui.atlas, &surface, field.as_deref());
            for out in typed {
                self.pad_osk_output(out);
            }
            return;
        }
        let size = surface.size;
        for touch in bridge::take_touches() {
            self.second_screen_touch(touch, events, id);
        }
        self.chat_keyboard_tick();
        let hosted = self.screen_hosted();
        let chat = self.chat_hosted();
        tabs::follow_screen(if hosted {
            Some(tabs::Tab::Inventory)
        } else if chat {
            Some(tabs::Tab::Chat)
        } else {
            None
        });
        // A screen shown here records its widgets for the touches as it draws: every frame.
        let due = hosted
            || chat
            || SHOWN.with_borrow(|shown| {
                shown
                    .as_ref()
                    .and_then(|s| s.drawn)
                    .is_none_or(|t| t.elapsed() >= INTERVAL)
            });
        if due {
            let batch = self.panel_hud(size, hosted);
            self.present_panel(&batch, surface.generation);
        }
    }

    /// Leave the second screen blank (the game is closing; the launcher draws nothing there).
    pub(super) fn blank_second_screen(&mut self) {
        let Some(generation) = SHOWN.with_borrow(|s| s.as_ref().map(|s| s.generation)) else {
            return;
        };
        let size = SHOWN.with_borrow(|s| s.as_ref().map(|s| s.panel.size()).unwrap_or([1, 1]));
        self.present_panel(&HudBatch::with_scale(size, 1), generation);
        bridge::close();
    }

    fn present_panel(&mut self, batch: &HudBatch, generation: u64) {
        if let Some(renderer) = self.renderer.as_ref() {
            present(renderer, batch, generation);
        }
    }

    fn second_screen_touch(&mut self, touch: Touch, events: &dyn EventLoopCtl, id: WindowId) {
        let Some(drawn) = host::drawn() else {
            return;
        };
        if self.player.is_none() {
            return;
        }
        let point = touch.at.map(|v| v / drawn.scale as f32);
        if host::screen_touch(self, &drawn, &touch, point, events, id)
            || chat::touch(self, &touch, point, events, id)
        {
            return;
        }
        if touch.phase != Phase::Started {
            return;
        }
        if self.panel_tabs_touch(point) {
            return;
        }
        // The hotbar is hit-tested where vanilla lays it out: the touch moves down with it.
        let hotbar = drawn.hotbar_scale as f32;
        let gui = drawn.size.map(|v| (v as f32 / hotbar).ceil());
        let shift = hotbar_shift(drawn.size, drawn.hotbar_scale);
        let at = [touch.at[0] / hotbar, touch.at[1] / hotbar - shift];
        if !self.spectator()
            && let Some(slot) = horizon_client::touch::hotbar_slot(gui[0], gui[1], at)
        {
            self.selected = slot;
        }
    }

    /// The panel: the vanilla hotbar along the top at its own larger scale, the tabs along the
    /// bottom, and the selected tab's content (or the `hosted` screen) between them.
    fn panel_hud(&mut self, size: [u32; 2], hosted: bool) -> HudBatch {
        let hotbar_scale = hotbar_scale(size);
        let screen = hosted
            .then(|| self.hosted_layout(size, hotbar_scale))
            .flatten();
        let scale = screen.as_ref().map_or_else(|| gui_scale(size), |s| s.scale);
        let mut batch = HudBatch::with_scale(size, scale);
        host::set_drawn(None);
        if self.player.is_none() || self.title.is_some() {
            return batch;
        }
        batch.game_time = ((self.world.game_time.rem_euclid(24000)) as f32
            + self.clock.fraction() as f32)
            / 24000.0;
        let top = content_top(hotbar_scale, scale);
        let bottom = tabs_bottom(size, scale);
        self.draw_panel_tabs(&mut batch, top, bottom, screen.is_some());
        // The hotbar as vanilla lays it out at the bottom, at its scale, moved up to the top edge.
        let mut hotbar = HudBatch::with_scale(size, hotbar_scale);
        hotbar.game_time = batch.game_time;
        self.draw_hotbar_hud(&mut hotbar);
        let k = hotbar_scale as f32 / scale as f32;
        let dy = hotbar_shift(size, hotbar_scale) * k;
        batch.append_scaled(hotbar, k, 0.0, dy);
        let offset = screen.map(|screen| self.draw_hosted(&mut batch, screen, top, bottom));
        host::set_drawn(Some(host::Drawn {
            scale,
            size,
            hotbar_scale,
            top,
            bottom,
            offset,
        }));
        batch
    }
}

/// Follow the second screen's Surface: a panel on it for `renderer`, made when the Surface or the
/// renderer is new. The Surface while a panel is shown on it; touches with no panel are dropped.
fn follow_surface(renderer: Option<&Renderer>) -> Option<bridge::Surface> {
    let surface = bridge::surface();
    let current = SHOWN.with_borrow(|shown| {
        shown.as_ref().is_some_and(|shown| {
            surface.as_ref().map(|s| s.generation) == Some(shown.generation)
                && renderer.is_some_and(|r| r.owns_panel(&shown.panel))
        })
    });
    if !current {
        bridge::close();
    }
    let (Some(surface), Some(renderer)) = (surface, renderer) else {
        bridge::take_touches();
        return None;
    };
    if !current && FAILED.get() != surface.generation {
        match bridge::panel_surface(renderer.instance(), &surface)
            .and_then(|made| renderer.panel(made, surface.size))
        {
            Ok(panel) => {
                log::info!(
                    "Second screen: {}x{} panel at GUI scale {}",
                    surface.size[0],
                    surface.size[1],
                    gui_scale(surface.size)
                );
                // Touches from before the panel showed (the launcher's) are not for it.
                bridge::take_touches();
                bridge::drawing(Some(surface.generation));
                SHOWN.set(Some(Shown {
                    panel,
                    generation: surface.generation,
                    _window: surface.window.clone(),
                    drawn: None,
                }));
            }
            Err(error) => {
                FAILED.set(surface.generation);
                log::warn!("Second screen: no panel ({error})");
            }
        }
    }
    if SHOWN.with_borrow(Option::is_none) {
        bridge::take_touches();
        return None;
    }
    Some(surface)
}

/// Draw `batch` on the panel of `generation`.
fn present(renderer: &Renderer, batch: &HudBatch, generation: u64) {
    let size = bridge::surface()
        .filter(|s| s.generation == generation)
        .map(|s| s.size);
    let result = SHOWN.with_borrow_mut(|shown| {
        let shown = shown.as_mut()?;
        if let Some(size) = size {
            shown.panel.resize(size);
        }
        shown.drawn = Some(Instant::now());
        Some(renderer.draw_panel(&mut shown.panel, batch, wgpu::Color::BLACK))
    });
    if let Some(Err(error)) = result {
        log::warn!("Second screen: {error}");
        FAILED.set(generation);
        bridge::close();
    }
}

/// Once a frame of the menus shown before a world loads (`startup_menu`): `menu_panel`.
pub(super) fn startup_panel(
    renderer: Option<&Renderer>,
    atlas: &HudAtlas,
    field: Option<&str>,
) -> Vec<Output> {
    match (follow_surface(renderer), renderer) {
        (Some(surface), Some(renderer)) => menu_panel(renderer, atlas, &surface, field),
        _ => Vec::new(),
    }
}

/// The text box and the keyboard under a menu, on a panel `size` pixels big at GUI `scale`.
fn menu_areas(size: [u32; 2], scale: u32) -> ([f32; 4], [f32; 4]) {
    const PADDING: f32 = 6.0;
    const BOX_H: f32 = 20.0;
    let w = (size[0] as f32 / scale as f32).floor();
    let h = (size[1] as f32 / scale as f32).floor();
    let text = [PADDING, PADDING, w - 2.0 * PADDING, BOX_H];
    let top = PADDING + BOX_H + 2.0 * keyboard::GAP;
    let keys = [
        PADDING,
        top,
        w - 2.0 * PADDING,
        (h - PADDING - top).max(keyboard::HEIGHT),
    ];
    (text, keys)
}

/// The panel under a menu with no world (the title screens, before a world or after one): while
/// a text field takes typing (`field`, its text), the field's text and a keyboard; blank
/// otherwise. What the keys typed, as the system keyboard's.
fn menu_panel(
    renderer: &Renderer,
    atlas: &HudAtlas,
    surface: &bridge::Surface,
    field: Option<&str>,
) -> Vec<Output> {
    let size = surface.size;
    let scale = gui_scale(size);
    let (text_box, keys) = menu_areas(size, scale);
    let mut typed = Vec::new();
    for touch in bridge::take_touches() {
        match touch.phase {
            Phase::Started if field.is_some() => {
                let point = touch.at.map(|v| v / scale as f32);
                if let Some(key) = keyboard::press(keys, point, keyboard::Layout::Field) {
                    typed.push(match key {
                        keyboard::Typed::Text(text) => Output::Text(text),
                        keyboard::Typed::Delete => Output::Backspace,
                        keyboard::Typed::History | keyboard::Typed::Send => Output::Enter,
                    });
                }
            }
            Phase::Ended | Phase::Cancelled => keyboard::release(),
            _ => {}
        }
    }
    if field.is_none() {
        keyboard::release();
    } else if keyboard::repeat() {
        typed.push(Output::Backspace);
    }
    // Redrawn while it changes; otherwise now and then, for a field that closed.
    let due = field.is_some()
        || SHOWN.with_borrow(|shown| {
            shown
                .as_ref()
                .and_then(|s| s.drawn)
                .is_none_or(|t| t.elapsed() >= Duration::from_millis(250))
        });
    if due {
        let mut batch = HudBatch::with_scale(size, scale);
        if let Some(text) = field {
            let [x, y, w, h] = text_box;
            horizon_client::ui::nine_slice(
                &mut batch,
                atlas,
                "gui/sprites/widget/text_field_highlighted",
                x,
                y,
                w,
                h,
                u32::MAX,
            );
            // The end of the text, with the cursor after it, as much as fits.
            let room = w - 10.0;
            let mut shown: String = text.to_owned();
            while !shown.is_empty() && HudBatch::text_width(atlas, &format!("{shown}_")) > room {
                shown.remove(0);
            }
            batch.text(
                atlas,
                &format!("{shown}_"),
                x + 5.0,
                y + (h - 8.0) / 2.0,
                0xFFE0_E0E0,
                true,
            );
            keyboard::draw(&mut batch, atlas, keys, keyboard::Layout::Field);
        }
        present(renderer, &batch, surface.generation);
    }
    typed
}

//! The second screen's panel (`crate::second_screen`: the AYN Thor's lower screen). The hotbar
//! moves there, enlarged along its bottom edge within reach, under vanilla-styled tabs (`tabs`:
//! map, inventory, chat); the top screen keeps the world and what is drawn over it (crosshair,
//! the bars where the hotbar was, effects, titles, chat, every other screen). A tap on a hotbar
//! slot selects it. The inventory and every container screen open on the panel instead
//! (`host`), touched directly. Without a second screen nothing changes.
use super::{Game, event_loop::EventLoopCtl};
use crate::second_screen::{self as bridge, FAILED, Phase, SHOWN, Shown, Touch};
use horizon_client::hud::HudBatch;
use std::time::{Duration, Instant};
use winit::window::WindowId;

mod chat;
mod host;
mod tabs;
pub(super) use tabs::map_blocks_per_pixel;

/// The panel redraws at most this often: it shows status that changes slowly next to the world.
const INTERVAL: Duration = Duration::from_millis(16);
/// The panel's GUI keeps at least this many GUI pixels, so the hotbar is large enough to tap.
const MIN_GUI: [u32; 2] = [300, 200];

/// The hotbar's GUI pixels across the panel: the hotbar (182) with room for the off hand slot
/// or the attack indicator (29) on either side.
const HOTBAR_ROOM: u32 = 182 + 2 * 29;
/// The panel content's distance from the top edge.
const TOP: f32 = 2.0;

/// The hotbar's own GUI scale: the largest that fits `HOTBAR_ROOM` across the panel.
fn hotbar_scale(size: [u32; 2]) -> u32 {
    (size[0] / HOTBAR_ROOM).max(1)
}

/// The top of the hotbar (its selected slot's frame, 23 high) in panel pixels.
fn hotbar_top(size: [u32; 2], scale: u32) -> f32 {
    ((size[1] as f32 / scale as f32).ceil() - 23.0) * scale as f32
}

/// The tabs' bottom edge, one GUI pixel above the hotbar, at the panel's GUI `scale`.
fn tabs_bottom(hotbar_top: f32, scale: u32) -> f32 {
    (hotbar_top / scale as f32).floor() - 1.0
}

/// The selected tab's panel (GUI pixels) and the panel's GUI scale, on a panel `size` pixels
/// big showing no screen of the game's.
fn tab_panel(size: [u32; 2]) -> ([f32; 4], u32) {
    let scale = gui_scale(size);
    let bottom = tabs_bottom(hotbar_top(size, hotbar_scale(size)), scale);
    let width = (size[0] as f32 / scale as f32).ceil();
    (tabs::panel_rect(width, TOP, bottom), scale)
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
        let surface = bridge::surface();
        let current = SHOWN.with_borrow(|shown| {
            shown.as_ref().is_some_and(|shown| {
                surface.as_ref().map(|s| s.generation) == Some(shown.generation)
                    && self
                        .renderer
                        .as_ref()
                        .is_some_and(|r| r.owns_panel(&shown.panel))
            })
        });
        if !current {
            bridge::close();
        }
        let (Some(surface), Some(renderer)) = (surface, self.renderer.as_ref()) else {
            bridge::take_touches();
            return;
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
        let Some(renderer) = self.renderer.as_ref() else {
            return;
        };
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
        let hotbar = drawn.hotbar_scale as f32;
        let gui = drawn.size.map(|v| (v as f32 / hotbar).ceil());
        if !self.spectator()
            && let Some(slot) =
                horizon_client::touch::hotbar_slot(gui[0], gui[1], touch.at.map(|v| v / hotbar))
        {
            self.selected = slot;
        }
    }

    /// The panel: the vanilla hotbar along the bottom at its own larger scale, the tabs above
    /// it, and the selected tab's content (or the `hosted` screen) above them.
    fn panel_hud(&mut self, size: [u32; 2], hosted: bool) -> HudBatch {
        let hotbar_scale = hotbar_scale(size);
        let hotbar_top = hotbar_top(size, hotbar_scale);
        let screen = hosted
            .then(|| self.hosted_layout(size, hotbar_top))
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
        let bottom = tabs_bottom(hotbar_top, scale);
        self.draw_panel_tabs(&mut batch, TOP, bottom, screen.is_some());
        // The hotbar as vanilla lays it out at the bottom, drawn over the same pixels at its scale.
        let mut hotbar = HudBatch::with_scale(size, hotbar_scale);
        hotbar.game_time = batch.game_time;
        self.draw_hotbar_hud(&mut hotbar);
        batch.append_scaled(hotbar, hotbar_scale as f32 / scale as f32, 0.0, 0.0);
        let offset = screen.map(|screen| self.draw_hosted(&mut batch, screen, TOP, bottom));
        host::set_drawn(Some(host::Drawn {
            scale,
            size,
            hotbar_scale,
            bottom,
            offset,
        }));
        batch
    }
}

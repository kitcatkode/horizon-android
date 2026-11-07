//! The second screen's tabs, drawn as vanilla's: the creative inventory's tab sprites with an
//! item for each, over a panel cut from the inventory's own background, so resource packs dress
//! them too. Map: the minimap on its paper with zoom buttons and the player's place. Inventory:
//! the open inventory or container screen itself (`super::host`); with none open, the armour,
//! off hand and main inventory. Chat: the chat with a keyboard (`super::chat`).
use super::super::{Game, Screen};
use horizon_client::{
    extras_hud,
    hud::{HudBatch, MINIMAP_SPRITE, Rect},
    items::ItemStack,
};
use std::{cell::RefCell, sync::Arc};
use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tab {
    #[default]
    Map,
    Inventory,
    Chat,
}
impl Tab {
    const ALL: [Tab; 3] = [Tab::Map, Tab::Inventory, Tab::Chat];
    fn icon(self) -> &'static str {
        match self {
            Tab::Map => "minecraft:filled_map",
            Tab::Inventory => "minecraft:chest",
            Tab::Chat => "minecraft:writable_book",
        }
    }
}

/// The map's zoom levels, in blocks per map pixel (vanilla maps' scales 0 to 3).
const ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

struct Ui {
    tab: Tab,
    zoom: usize,
    /// Where the last frame drew everything, for the touches before the next.
    layout: Option<Layout>,
    /// The tab of the open screen the panel shows (the inventory's, the chat's).
    hosting: Option<Tab>,
    /// The tab to go back to when the screen closes.
    before: Option<Tab>,
    /// The screen took its tab over from another one.
    took_over: bool,
}
thread_local! {
    static UI: RefCell<Ui> = const {
        RefCell::new(Ui {
            tab: Tab::Map,
            zoom: 1,
            layout: None,
            hosting: None,
            before: None,
            took_over: false,
        })
    };
}

/// The built-in map's scale on the panel (`Game::minimap_wanted`).
pub(in crate::game) fn map_blocks_per_pixel() -> f32 {
    UI.with_borrow(|ui| ZOOMS[ui.zoom])
}

/// The tabs follow the open screen the panel shows (`hosting`: its tab): its tab is selected
/// while it is open, and when it closes the tab before it comes back if the screen took over
/// (or it was the inventory's, which shows little without it).
pub(super) fn follow_screen(hosting: Option<Tab>) {
    UI.with_borrow_mut(|ui| {
        if ui.hosting == hosting {
            return;
        }
        if let Some(old) = ui.hosting
            && ui.tab == old
            && (ui.took_over || old == Tab::Inventory)
            && let Some(before) = ui.before.take()
        {
            ui.tab = before;
        }
        ui.hosting = hosting;
        ui.took_over = false;
        if let Some(new) = hosting
            && ui.tab != new
        {
            ui.before = Some(std::mem::replace(&mut ui.tab, new));
            ui.took_over = true;
        }
    });
}

/// The selected tab.
pub(super) fn selected() -> Tab {
    UI.with_borrow(|ui| ui.tab)
}

/// The selected tab's panel from `top` with the tabs ending at `bottom`.
pub(super) fn panel_rect(width: f32, top: f32, bottom: f32) -> [f32; 4] {
    Layout::new(width, top, bottom).panel
}

/// Whether `point` (GUI pixels, as last drawn) is on a tab.
pub(super) fn on_tabs(point: [f32; 2]) -> bool {
    UI.with_borrow(|ui| {
        ui.layout
            .is_some_and(|l| (0..Tab::ALL.len()).any(|i| inside(l.tab(i), point)))
    })
}

/// Where the open screen goes: from `top` down to the tabs, which end at `bottom`.
pub(super) fn screen_area(width: f32, top: f32, bottom: f32) -> [f32; 4] {
    let layout = Layout::new(width, top, bottom);
    let [x, y, w, _] = layout.panel;
    [x, y, w, layout.tabs_y - y]
}

/// The tabs' height under the open screen.
pub(super) const TABS_HEIGHT: f32 = TAB[1];

/// A creative inventory tab: 26 by 32 (stretched across), overlapping the panel by 4.
const TAB: [f32; 2] = [26.0, 32.0];
const LABEL: u32 = 0xFF40_4040;
/// The map tab's zoom buttons.
const BUTTON: f32 = 20.0;

#[derive(Clone, Copy, Debug)]
struct Layout {
    /// x, y, width, height.
    panel: [f32; 4],
    tabs_y: f32,
}
impl Layout {
    /// The panel from `top` (under the hotbar), the tabs under it ending at `bottom`.
    fn new(width: f32, top: f32, bottom: f32) -> Self {
        let tabs_y = bottom - TAB[1];
        let panel_bottom = (tabs_y + 4.0).max(top + 40.0);
        Self {
            panel: [2.0, top, width - 4.0, panel_bottom - top],
            tabs_y,
        }
    }
    /// Tab `i`: the tabs share the panel's width equally.
    fn tab(&self, i: usize) -> [f32; 4] {
        let [x, _, w, _] = self.panel;
        let n = Tab::ALL.len();
        let each = (w / n as f32).floor();
        let tx = x + each * i as f32;
        let tw = if i + 1 == n { x + w - tx } else { each };
        [tx, self.tabs_y, tw, TAB[1]]
    }
    /// The map tab's zoom in and zoom out buttons.
    fn zoom_buttons(&self) -> [[f32; 4]; 2] {
        let [x, y, w, _] = self.panel;
        let bx = x + w - 8.0 - BUTTON;
        [
            [bx, y + 8.0, BUTTON, BUTTON],
            [bx, y + 10.0 + BUTTON, BUTTON, BUTTON],
        ]
    }
}

fn inside([x, y, w, h]: [f32; 4], p: [f32; 2]) -> bool {
    (x..x + w).contains(&p[0]) && (y..y + h).contains(&p[1])
}

/// Part of a sprite, in its texture's pixels.
fn part(r: Rect, x: f32, y: f32, w: f32, h: f32) -> Rect {
    let (du, dv) = ((r.u1 - r.u0) / r.w as f32, (r.v1 - r.v0) / r.h as f32);
    let u0 = r.u0 + x * du;
    let v0 = r.v0 + y * dv;
    Rect {
        u0,
        v0,
        u1: u0 + w * du,
        v1: v0 + h * dv,
        w: w as u32,
        h: h as u32,
    }
}

/// A container panel of any size, nine-sliced from the inventory's background (176 by 166,
/// 4-pixel rounded border).
fn vanilla_panel(batch: &mut HudBatch, inventory: Rect, [x, y, w, h]: [f32; 4]) {
    const B: f32 = 4.0;
    let (tw, th) = (176.0, 166.0);
    let columns = [
        (0.0, B, x, B),
        (B, tw - 2.0 * B, x + B, w - 2.0 * B),
        (tw - B, B, x + w - B, B),
    ];
    let rows = [
        (0.0, B, y, B),
        (B, th - 2.0 * B, y + B, h - 2.0 * B),
        (th - B, B, y + h - B, B),
    ];
    for (sy, sh, dy, dh) in rows {
        for (sx, sw, dx, dw) in columns {
            // The middle of the inventory has slots: its plain grey is the first border pixel in.
            let (sx, sw) = if sw > B { (B, 1.0) } else { (sx, sw) };
            let (sy, sh) = if sh > B { (B, 1.0) } else { (sy, sh) };
            batch.rect_uv(dx, dy, dw, dh, part(inventory, sx, sy, sw, sh), u32::MAX);
        }
    }
}

/// A tab sprite stretched to `w`: its 4-pixel ends kept, its middle widened.
fn stretched_tab(batch: &mut HudBatch, tab: Rect, [x, y, w, h]: [f32; 4]) {
    const E: f32 = 4.0;
    let (sw, sh) = (tab.w as f32, tab.h as f32);
    batch.rect_uv(x, y, E, h, part(tab, 0.0, 0.0, E, sh), u32::MAX);
    batch.rect_uv(
        x + E,
        y,
        w - 2.0 * E,
        h,
        part(tab, E, 0.0, sw - 2.0 * E, sh),
        u32::MAX,
    );
    batch.rect_uv(x + w - E, y, E, h, part(tab, sw - E, 0.0, E, sh), u32::MAX);
}

impl Game<'_> {
    /// The tabs, ending at `bottom` (the hotbar is under them), and the selected one's panel
    /// from `top` down to them. With a screen `hosted` there, only the tabs: the screen goes
    /// above them.
    pub(in crate::game) fn draw_panel_tabs(
        &mut self,
        batch: &mut HudBatch,
        top: f32,
        bottom: f32,
        hosted: bool,
    ) {
        let layout = Layout::new(batch.width, top, bottom);
        let tab = UI.with_borrow_mut(|ui| {
            ui.layout = Some(layout);
            ui.tab
        });
        let selected = Tab::ALL.iter().position(|t| *t == tab).unwrap_or(0);
        let n = Tab::ALL.len();
        // The creative inventory's bottom tabs: the first and last join the panel's corners.
        let sprite = |on: bool, i: usize| {
            let column = if i == 0 {
                1
            } else if i + 1 == n {
                7
            } else {
                2
            };
            let state = if on { "selected" } else { "unselected" };
            format!("gui/sprites/container/creative_inventory/tab_bottom_{state}_{column}")
        };
        let draw_tab = |batch: &mut HudBatch, on: bool, i: usize| {
            if let Some(r) = self.gui.atlas.sprites.get(&sprite(on, i)).copied() {
                stretched_tab(batch, r, layout.tab(i));
            }
        };
        // Unselected tabs behind the panel, then the panel, then the selected tab joining it.
        for i in (0..n).filter(|i| *i != selected) {
            draw_tab(batch, false, i);
        }
        if !hosted
            && let Some(inventory) = self
                .gui
                .atlas
                .sprites
                .get("gui/container/inventory")
                .copied()
        {
            vanilla_panel(batch, inventory, layout.panel);
        }
        draw_tab(batch, true, selected);
        for (i, t) in Tab::ALL.iter().enumerate() {
            let [x, y, w, _] = layout.tab(i);
            let label = match t {
                Tab::Map => self.translate("item.minecraft.filled_map", "Map"),
                Tab::Inventory => self.translate("container.inventory", "Inventory"),
                Tab::Chat => "Chat".to_owned(),
            };
            let text = HudBatch::text_width(&self.gui.atlas, &label);
            let label = (16.0 + 4.0 + text <= w - 8.0).then_some(label);
            let content = 16.0 + label.as_ref().map_or(0.0, |_| 4.0 + text);
            let ix = (x + (w - content) / 2.0).floor();
            let stack = ItemStack {
                item: Arc::from(t.icon()),
                count: 1,
                damage: 0,
                extra: None,
            };
            self.draw_item(batch, &stack, ix, y + 7.0);
            if let Some(label) = label {
                batch.text(&self.gui.atlas, &label, ix + 20.0, y + 11.0, LABEL, false);
            }
        }
        match tab {
            _ if hosted => {}
            Tab::Map => self.draw_map_tab(batch, &layout),
            Tab::Inventory => self.draw_inventory_tab(batch, &layout),
            Tab::Chat => self.draw_chat_tab(batch, layout.panel),
        }
    }

    /// A tap at `point` (GUI pixels) on the tabs or a tab's buttons; false when it is elsewhere.
    pub(in crate::game) fn panel_tabs_touch(&mut self, point: [f32; 2]) -> bool {
        let Some(layout) = UI.with_borrow(|ui| ui.layout) else {
            return false;
        };
        if let Some(i) = (0..Tab::ALL.len()).find(|i| inside(layout.tab(*i), point)) {
            let tab = Tab::ALL[i];
            let old = UI.with_borrow_mut(|ui| {
                let old = std::mem::replace(&mut ui.tab, tab);
                if tab == Tab::Inventory && old != tab {
                    ui.before = Some(old);
                }
                old
            });
            if old != tab {
                self.audio.play_button_click();
                // The Inventory tab opens the inventory as key.inventory does, and leaving it
                // closes the screen it shows.
                let result = if tab == Tab::Inventory && self.screen == Screen::Playing {
                    self.key_pressed(KeyCode::KeyE)
                } else if (old == Tab::Inventory
                    && matches!(self.screen, Screen::Inventory | Screen::Creative))
                    || (old == Tab::Chat && self.screen == Screen::Chat)
                {
                    self.pad_back()
                } else {
                    Ok(())
                };
                if let Err(error) = result {
                    log::warn!("Second screen tab: {error}");
                }
            }
            return true;
        }
        if UI.with_borrow(|ui| ui.tab) == Tab::Map {
            let [zoom_in, zoom_out] = layout.zoom_buttons();
            let step = if inside(zoom_in, point) {
                Some(-1)
            } else if inside(zoom_out, point) {
                Some(1)
            } else {
                None
            };
            if let Some(step) = step {
                let moved = UI.with_borrow_mut(|ui| {
                    let zoom = ui.zoom.saturating_add_signed(step).min(ZOOMS.len() - 1);
                    std::mem::replace(&mut ui.zoom, zoom) != zoom
                });
                if moved {
                    self.audio.play_button_click();
                }
                return true;
            }
        }
        inside(layout.panel, point)
    }

    fn draw_map_tab(&mut self, batch: &mut HudBatch, layout: &Layout) {
        let [px, py, pw, ph] = layout.panel;
        let atlas = &self.gui.atlas;
        // The place on the left, as F3 shows it.
        let place = self.panel_place();
        let mut column = 8.0_f32;
        for (i, line) in place.iter().enumerate() {
            batch.text(
                atlas,
                line,
                px + 8.0,
                py + 10.0 + 10.0 * i as f32,
                LABEL,
                false,
            );
            column = column.max(HudBatch::text_width(atlas, line) + 12.0);
        }
        // The zoom buttons on the right: in, out, and the scale under them.
        let zoom = UI.with_borrow(|ui| ui.zoom);
        for (i, [bx, by, bw, bh]) in layout.zoom_buttons().into_iter().enumerate() {
            let usable = if i == 0 {
                zoom > 0
            } else {
                zoom + 1 < ZOOMS.len()
            };
            let sprite = if usable {
                "gui/sprites/widget/button"
            } else {
                "gui/sprites/widget/button_disabled"
            };
            horizon_client::ui::nine_slice(batch, atlas, sprite, bx, by, bw, bh, u32::MAX);
            let label = if i == 0 { "+" } else { "-" };
            let tw = HudBatch::text_width(atlas, label);
            let color = if usable { u32::MAX } else { 0xFFA0_A0A0 };
            batch.text(atlas, label, bx + (bw - tw) / 2.0, by + 6.0, color, true);
        }
        let scale = format!("1:{}", ZOOMS[zoom]);
        let [bx, by, bw, bh] = layout.zoom_buttons()[1];
        let tw = HudBatch::text_width(atlas, &scale);
        batch.text(
            atlas,
            &scale,
            bx + (bw - tw) / 2.0,
            by + bh + 4.0,
            LABEL,
            false,
        );
        let side_room = column.max(BUTTON + 16.0);
        // The map on its paper (as held: 128 of 142 units), centred.
        let paper = (ph - 8.0).min(pw - 2.0 * side_room).floor();
        if paper < 40.0 {
            return;
        }
        let x = (px + (pw - paper) / 2.0).floor();
        let y = (py + (ph - paper) / 2.0).floor();
        if let Some(background) = atlas.sprites.get("map/map_background").copied() {
            batch.rect_uv(x, y, paper, paper, background, u32::MAX);
        }
        if let Some((_, _, view)) = self.minimap_shown()
            && self.minimap_ready()
            && let Some(image) = atlas.sprites.get(MINIMAP_SPRITE).copied()
        {
            let side = (paper * 128.0 / 142.0).floor();
            let inset = ((paper - side) / 2.0).floor();
            let (arrow, north) = if view.rotate {
                (0.0, Some(view.north()))
            } else {
                (180.0 + view.yaw, None)
            };
            let look = extras_hud::MinimapLook {
                corner: horizon_client::extras::Corner::TopLeft,
                size: side,
                image,
                arrow,
                north,
            };
            extras_hud::draw_minimap_at(batch, atlas, x + inset, y + inset, &look);
        }
    }

    fn draw_inventory_tab(&mut self, batch: &mut HudBatch, layout: &Layout) {
        let Some(inventory) = self
            .gui
            .atlas
            .sprites
            .get("gui/container/inventory")
            .copied()
        else {
            return;
        };
        let [px, py, pw, _] = layout.panel;
        // The armour column with the off hand under it, and the main inventory beside it, cut
        // from the inventory's background (the hotbar is under the panel already).
        let width = 18.0 + 6.0 + 162.0;
        let x = (px + (pw - width) / 2.0).floor();
        let y = py + 8.0;
        batch.rect_uv(
            x,
            y,
            18.0,
            72.0,
            part(inventory, 7.0, 7.0, 18.0, 72.0),
            u32::MAX,
        );
        batch.rect_uv(
            x,
            y + 76.0,
            18.0,
            18.0,
            part(inventory, 76.0, 61.0, 18.0, 18.0),
            u32::MAX,
        );
        batch.rect_uv(
            x + 24.0,
            y,
            162.0,
            54.0,
            part(inventory, 7.0, 83.0, 162.0, 54.0),
            u32::MAX,
        );
        let atlas = &self.gui.atlas;
        for (i, icon) in ["helmet", "chestplate", "leggings", "boots"]
            .iter()
            .enumerate()
        {
            let at = (x + 1.0, y + 1.0 + 18.0 * i as f32);
            match &self.armor[i] {
                Some(stack) => self.draw_item(batch, stack, at.0, at.1),
                None => batch.sprite(
                    atlas,
                    &format!("gui/sprites/container/slot/{icon}"),
                    at.0,
                    at.1,
                ),
            }
        }
        match &self.offhand {
            Some(stack) => self.draw_item(batch, stack, x + 1.0, y + 77.0),
            None => batch.sprite(
                atlas,
                "gui/sprites/container/slot/shield",
                x + 1.0,
                y + 77.0,
            ),
        }
        for row in 0..3 {
            for col in 0..9 {
                if let Some(stack) = &self.inventory[9 + row * 9 + col] {
                    let sx = x + 25.0 + 18.0 * col as f32;
                    let sy = y + 1.0 + 18.0 * row as f32;
                    self.draw_item(batch, stack, sx, sy);
                }
            }
        }
    }

    /// Where the player is: the block position and facing (only where F3 may show them).
    fn panel_place(&self) -> Vec<String> {
        let Some(p) = self.player.as_ref() else {
            return Vec::new();
        };
        let yaw = self.yaw.rem_euclid(360.0);
        let facing = ["South", "West", "North", "East"][(((yaw + 45.0) / 90.0) as usize) % 4];
        let mut lines = Vec::new();
        if !self.reduced_debug_info() {
            let block = [p.position.x, p.position.y, p.position.z].map(|v| v.floor() as i64);
            lines.push(format!("X {}", block[0]));
            lines.push(format!("Y {}", block[1]));
            lines.push(format!("Z {}", block[2]));
        }
        lines.push(facing.to_string());
        lines
    }
}

//! The launcher's Material 3 frame: a navigation rail down the left edge (a navigation bar
//! along the bottom on a portrait screen) with the account as its floating action button, and
//! a top app bar with the page's title over the page.
use super::pages::Action;
use crate::catalog::Page;
use horizon_client::{
    ui_material::{self as m3, Button, State, icon, scheme},
    ui_theme::Ui,
};

/// The rail's width and the bar's height, in dp.
pub(super) const RAIL: f32 = 88.0;
pub(super) const BAR: f32 = 76.0;
/// The top app bar's height: the page's own content starts under it.
pub(super) const APP_BAR: f32 = 72.0;

/// Where the navigation left the page: its left edge and its height.
pub(super) struct Frame {
    pub left: f32,
    pub bottom: f32,
}

fn page_icon(page: Page) -> char {
    match page {
        Page::Play => icon::PLAY_ARROW,
        Page::Mods => icon::EXTENSION,
        Page::Packs => icon::PALETTE,
        Page::Shaders => icon::AUTO_AWESOME,
        Page::Installed => icon::FOLDER,
        Page::Settings => icon::SETTINGS,
        Page::Skins => icon::CHECKROOM,
    }
}

fn page_label(page: Page) -> &'static str {
    match page {
        Page::Installed => "Files",
        Page::Settings => "Setup",
        _ => page.name(),
    }
}

/// The page's title in the top app bar.
fn page_title(page: Page) -> &'static str {
    match page {
        Page::Play => "Play",
        Page::Mods => "Mods",
        Page::Packs => "Resource packs",
        Page::Shaders => "Shaders",
        Page::Installed => "Files",
        Page::Settings => "Setup",
        Page::Skins => "Skins",
    }
}

/// A destination: the active indicator (a 56 by 32 pill) behind its icon, the label under it.
fn destination(ui: &mut Ui, cx: f32, y: f32, page: Page, active: bool) {
    let s = scheme();
    if active {
        ui.rounded(cx - 28.0, y, 56.0, 32.0, 16.0, s.secondary_container);
    }
    let icon_color = if active {
        s.on_secondary_container
    } else {
        s.on_surface_variant
    };
    ui.icon(page_icon(page), cx - 12.0, y + 4.0, 24.0, icon_color, active);
    let label = page_label(page);
    let ty = m3::LABEL_MEDIUM;
    let width = ui.type_width(ty, label);
    let color = if active {
        s.on_surface
    } else {
        s.on_surface_variant
    };
    ui.type_text(ty, cx - width / 2.0, y + 36.0, color, label);
}

/// Draw the navigation and the top app bar for `current`, adding their targets.
pub(super) fn draw(
    ui: &mut Ui,
    targets: &mut Vec<([f32; 4], Action)>,
    current: Page,
    version: &str,
) -> Frame {
    let s = scheme();
    let (w, h) = (ui.w, ui.h);
    let portrait = h > w;
    let left = if portrait { 0.0 } else { RAIL };
    let bottom = if portrait { h - BAR } else { h };
    if portrait {
        // The navigation bar: the destinations share the width.
        ui.rounded(0.0, bottom, w, BAR, 0.0, s.surface_container);
        let each = w / Page::ALL.len() as f32;
        for (i, page) in Page::ALL.iter().enumerate() {
            let cx = each * (i as f32 + 0.5);
            destination(ui, cx, bottom + 12.0, *page, *page == current);
            targets.push(([each * i as f32, bottom, each, BAR], Action::Page(*page)));
        }
    } else {
        // The rail: the account's button, then the destinations.
        let cx = RAIL / 2.0;
        let fab = [cx - 28.0, 20.0, 56.0, 56.0];
        ui.rounded(fab[0], fab[1], fab[2], fab[3], m3::SHAPE_LARGE, s.primary_container);
        ui.icon(
            icon::ACCOUNT_CIRCLE,
            cx - 12.0,
            fab[1] + 16.0,
            24.0,
            s.on_primary_container,
            false,
        );
        targets.push((fab, Action::Account));
        let first = fab[1] + fab[3] + 24.0;
        let step = ((bottom - first - 8.0) / Page::ALL.len() as f32).min(68.0);
        for (i, page) in Page::ALL.iter().enumerate() {
            let y = first + step * i as f32;
            destination(ui, cx, y, *page, *page == current);
            targets.push(([0.0, y - 4.0, RAIL, 56.0], Action::Page(*page)));
        }
    }
    // The top app bar: the title, the version at the end.
    let title = page_title(current);
    ui.type_text(m3::HEADLINE_SMALL, left + 32.0, 22.0, s.on_surface, title);
    let ty = m3::BODY_SMALL;
    let version_w = ui.type_width(ty, version);
    let right = w - 24.0;
    if portrait {
        // No rail: the account's button sits at the end of the bar.
        let rect = [right - 40.0, 18.0, 40.0, 40.0];
        ui.m3_button(rect, Button::Tonal, None, "", State::default());
        ui.icon(
            icon::ACCOUNT_CIRCLE,
            rect[0] + 8.0,
            rect[1] + 8.0,
            24.0,
            s.on_secondary_container,
            false,
        );
        targets.push((rect, Action::Account));
    } else if left + 32.0 + ui.type_width(m3::HEADLINE_SMALL, title) + 24.0 < right - version_w {
        ui.type_text(ty, right - version_w, 30.0, s.on_surface_variant, version);
    }
    Frame { left, bottom }
}

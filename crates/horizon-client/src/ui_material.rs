//! Material 3 for Horizon's own interface (the Android launcher and the in-game overlays drawn
//! with [`crate::ui_theme`]): the dark colour scheme's roles, the type scale, the Material
//! Symbols icons and the components, in the launcher's dp.
//!
//! The scheme is the platform's when it has one (Android's dynamic colour, from the wallpaper:
//! [`set_scheme`]); otherwise Material 3's baseline dark scheme.
use crate::ui_theme::{Face, Ui};
use std::sync::RwLock;

/// An Android `0xAARRGGBB` colour in the HUD's encoding (`0xAABBGGRR`).
pub const fn argb(c: u32) -> u32 {
    (c & 0xff00_ff00) | (c >> 16 & 0xff) | (c & 0xff) << 16
}

/// The dark colour scheme's roles, in the HUD's encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scheme {
    pub primary: u32,
    pub on_primary: u32,
    pub primary_container: u32,
    pub on_primary_container: u32,
    pub secondary: u32,
    pub on_secondary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,
    pub tertiary: u32,
    pub on_tertiary: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,
    pub error: u32,
    pub on_error: u32,
    pub error_container: u32,
    pub on_error_container: u32,
    pub surface: u32,
    pub on_surface: u32,
    pub on_surface_variant: u32,
    pub surface_container_lowest: u32,
    pub surface_container_low: u32,
    pub surface_container: u32,
    pub surface_container_high: u32,
    pub surface_container_highest: u32,
    pub outline: u32,
    pub outline_variant: u32,
    pub inverse_surface: u32,
    pub inverse_on_surface: u32,
    pub inverse_primary: u32,
}

/// The roles in the order [`Scheme::from_roles`] and the Android bridge (`MaterialColors.java`)
/// list them.
pub const ROLES: [&str; 29] = [
    "primary",
    "on_primary",
    "primary_container",
    "on_primary_container",
    "secondary",
    "on_secondary",
    "secondary_container",
    "on_secondary_container",
    "tertiary",
    "on_tertiary",
    "tertiary_container",
    "on_tertiary_container",
    "error",
    "on_error",
    "error_container",
    "on_error_container",
    "surface",
    "on_surface",
    "on_surface_variant",
    "surface_container_lowest",
    "surface_container_low",
    "surface_container",
    "surface_container_high",
    "surface_container_highest",
    "outline",
    "outline_variant",
    "inverse_surface",
    "inverse_on_surface",
    "inverse_primary",
];

impl Scheme {
    /// Material 3's baseline dark scheme (source colour #6750A4).
    pub const BASELINE: Self = Self {
        primary: argb(0xFFD0_BCFF),
        on_primary: argb(0xFF38_1E72),
        primary_container: argb(0xFF4F_378B),
        on_primary_container: argb(0xFFEA_DDFF),
        secondary: argb(0xFFCC_C2DC),
        on_secondary: argb(0xFF33_2D41),
        secondary_container: argb(0xFF4A_4458),
        on_secondary_container: argb(0xFFE8_DEF8),
        tertiary: argb(0xFFEF_B8C8),
        on_tertiary: argb(0xFF49_2532),
        tertiary_container: argb(0xFF63_3B48),
        on_tertiary_container: argb(0xFFFF_D8E4),
        error: argb(0xFFF2_B8B5),
        on_error: argb(0xFF60_1410),
        error_container: argb(0xFF8C_1D18),
        on_error_container: argb(0xFFF9_DEDC),
        surface: argb(0xFF14_1218),
        on_surface: argb(0xFFE6_E0E9),
        on_surface_variant: argb(0xFFCA_C4D0),
        surface_container_lowest: argb(0xFF0F_0D13),
        surface_container_low: argb(0xFF1D_1B20),
        surface_container: argb(0xFF21_1F26),
        surface_container_high: argb(0xFF2B_2930),
        surface_container_highest: argb(0xFF36_343B),
        outline: argb(0xFF93_8F99),
        outline_variant: argb(0xFF49_454F),
        inverse_surface: argb(0xFFE6_E0E9),
        inverse_on_surface: argb(0xFF32_2F35),
        inverse_primary: argb(0xFF67_50A4),
    };

    /// A scheme from Android colours (`0xAARRGGBB`) in [`ROLES`] order; a role the platform
    /// lacks (0, or a short list) keeps the baseline's.
    pub fn from_roles(colors: &[u32]) -> Self {
        let mut scheme = Self::BASELINE;
        let fields: [&mut u32; 29] = [
            &mut scheme.primary,
            &mut scheme.on_primary,
            &mut scheme.primary_container,
            &mut scheme.on_primary_container,
            &mut scheme.secondary,
            &mut scheme.on_secondary,
            &mut scheme.secondary_container,
            &mut scheme.on_secondary_container,
            &mut scheme.tertiary,
            &mut scheme.on_tertiary,
            &mut scheme.tertiary_container,
            &mut scheme.on_tertiary_container,
            &mut scheme.error,
            &mut scheme.on_error,
            &mut scheme.error_container,
            &mut scheme.on_error_container,
            &mut scheme.surface,
            &mut scheme.on_surface,
            &mut scheme.on_surface_variant,
            &mut scheme.surface_container_lowest,
            &mut scheme.surface_container_low,
            &mut scheme.surface_container,
            &mut scheme.surface_container_high,
            &mut scheme.surface_container_highest,
            &mut scheme.outline,
            &mut scheme.outline_variant,
            &mut scheme.inverse_surface,
            &mut scheme.inverse_on_surface,
            &mut scheme.inverse_primary,
        ];
        for (field, color) in fields.into_iter().zip(colors) {
            if *color != 0 {
                *field = argb(*color);
            }
        }
        scheme
    }
}

static SCHEME: RwLock<Scheme> = RwLock::new(Scheme::BASELINE);

/// The scheme every Material 3 surface draws with.
pub fn scheme() -> Scheme {
    *SCHEME.read().unwrap_or_else(|e| e.into_inner())
}

/// Use `scheme` from now on (the platform's dynamic colour changed).
pub fn set_scheme(scheme: Scheme) {
    *SCHEME.write().unwrap_or_else(|e| e.into_inner()) = scheme;
}

/// `color` at `opacity` (a state layer's, a disabled content's).
pub fn alpha(color: u32, opacity: f32) -> u32 {
    let a = ((color >> 24) as f32 * opacity.clamp(0.0, 1.0)).round() as u32;
    (color & 0x00ff_ffff) | a << 24
}

/// `top` over `base` at `opacity`, opaque: a state layer flattened onto its container.
pub fn over(base: u32, top: u32, opacity: f32) -> u32 {
    let mix = |shift: u32| {
        let b = (base >> shift & 0xff) as f32;
        let t = (top >> shift & 0xff) as f32;
        ((b + (t - b) * opacity).round() as u32) << shift
    };
    0xff00_0000 | mix(0) | mix(8) | mix(16)
}

/// The state layer's opacities.
pub const HOVER: f32 = 0.08;
pub const FOCUS: f32 = 0.10;
pub const PRESSED: f32 = 0.10;
/// Disabled content and containers.
pub const DISABLED_CONTENT: f32 = 0.38;
pub const DISABLED_CONTAINER: f32 = 0.12;

/// A step of the type scale: size and line height in dp, and its face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    pub size: f32,
    pub line: f32,
    pub face: Face,
}
const fn ty(size: f32, line: f32, face: Face) -> Type {
    Type { size, line, face }
}
pub const DISPLAY_SMALL: Type = ty(36.0, 44.0, Face::Regular);
pub const HEADLINE_LARGE: Type = ty(32.0, 40.0, Face::Regular);
pub const HEADLINE_MEDIUM: Type = ty(28.0, 36.0, Face::Regular);
pub const HEADLINE_SMALL: Type = ty(24.0, 32.0, Face::Regular);
pub const TITLE_LARGE: Type = ty(22.0, 28.0, Face::Regular);
pub const TITLE_MEDIUM: Type = ty(16.0, 24.0, Face::Medium);
pub const TITLE_SMALL: Type = ty(14.0, 20.0, Face::Medium);
pub const BODY_LARGE: Type = ty(16.0, 24.0, Face::Regular);
pub const BODY_MEDIUM: Type = ty(14.0, 20.0, Face::Regular);
pub const BODY_SMALL: Type = ty(12.0, 16.0, Face::Regular);
pub const LABEL_LARGE: Type = ty(14.0, 20.0, Face::Medium);
pub const LABEL_MEDIUM: Type = ty(12.0, 16.0, Face::Medium);
pub const LABEL_SMALL: Type = ty(11.0, 16.0, Face::Medium);

/// The shape scale's corner radii, in dp (`full` is half the height).
pub const SHAPE_EXTRA_SMALL: f32 = 4.0;
pub const SHAPE_SMALL: f32 = 8.0;
pub const SHAPE_MEDIUM: f32 = 12.0;
pub const SHAPE_LARGE: f32 = 16.0;
pub const SHAPE_EXTRA_LARGE: f32 = 28.0;

/// Material Symbols Rounded, by name (the bundled subset: `tools/material_symbols_subset.py`).
pub mod icon {
    pub const PLAY_ARROW: char = '\u{e037}';
    pub const EXTENSION: char = '\u{e87b}';
    pub const PALETTE: char = '\u{e40a}';
    pub const AUTO_AWESOME: char = '\u{e65f}';
    pub const FOLDER: char = '\u{e2c7}';
    pub const FOLDER_OPEN: char = '\u{e2c8}';
    pub const SETTINGS: char = '\u{e8b8}';
    pub const CHECKROOM: char = '\u{f19e}';
    pub const ACCOUNT_CIRCLE: char = '\u{f20b}';
    pub const PERSON: char = '\u{f0d3}';
    pub const SEARCH: char = '\u{ef7a}';
    pub const DOWNLOAD: char = '\u{f090}';
    pub const UPLOAD: char = '\u{f09b}';
    pub const CHECK: char = '\u{e668}';
    pub const CHECK_CIRCLE: char = '\u{f0be}';
    pub const CLOSE: char = '\u{e5cd}';
    pub const ADD: char = '\u{e145}';
    pub const DELETE: char = '\u{e92e}';
    pub const ARROW_BACK: char = '\u{e5c4}';
    pub const CHEVRON_LEFT: char = '\u{e5cb}';
    pub const CHEVRON_RIGHT: char = '\u{e5cc}';
    pub const EXPAND_MORE: char = '\u{e5cf}';
    pub const EXPAND_LESS: char = '\u{e5ce}';
    pub const REFRESH: char = '\u{e5d5}';
    pub const OPEN_IN_NEW: char = '\u{e89e}';
    pub const CONTENT_COPY: char = '\u{e14d}';
    pub const LOGOUT: char = '\u{e9ba}';
    pub const LOGIN: char = '\u{ea77}';
    pub const CLOUD: char = '\u{f15c}';
    pub const CLOUD_UPLOAD: char = '\u{e2c3}';
    pub const SPEED: char = '\u{e9e4}';
    pub const BUG_REPORT: char = '\u{e868}';
    pub const DESCRIPTION: char = '\u{e873}';
    pub const TUNE: char = '\u{e429}';
    pub const SCREEN_ROTATION: char = '\u{f2d5}';
    pub const MONITOR: char = '\u{ef5b}';
    pub const SPORTS_ESPORTS: char = '\u{ea28}';
    pub const ERROR: char = '\u{f8b6}';
    pub const INFO: char = '\u{e88e}';
    pub const ROTATE_LEFT: char = '\u{e419}';
    pub const ROTATE_RIGHT: char = '\u{e41a}';
    pub const ZOOM_IN: char = '\u{e8ff}';
    pub const ZOOM_OUT: char = '\u{e900}';
    pub const VISIBILITY: char = '\u{e8f4}';
    pub const VISIBILITY_OFF: char = '\u{e8f5}';
    pub const HISTORY: char = '\u{e8b3}';
    pub const MORE_VERT: char = '\u{e5d4}';
    pub const KEYBOARD: char = '\u{e312}';
    pub const BACKSPACE: char = '\u{e14a}';
    pub const SEND: char = '\u{e163}';
    pub const ARROW_UPWARD: char = '\u{e5d8}';
    pub const KEYBOARD_CAPSLOCK: char = '\u{e318}';
    pub const PUBLIC: char = '\u{e80b}';
    pub const STORAGE: char = '\u{e1db}';
    pub const MEMORY: char = '\u{e322}';
    pub const ASPECT_RATIO: char = '\u{e85b}';
    pub const FULLSCREEN: char = '\u{e5d0}';
    pub const LINK: char = '\u{e250}';
    pub const STAR: char = '\u{f09a}';
    pub const MAP: char = '\u{e55b}';
    pub const INVENTORY_2: char = '\u{e1a1}';
    pub const CHAT: char = '\u{e0c9}';
    pub const LANDSCAPE: char = '\u{e564}';
    pub const VIDEOCAM: char = '\u{e04b}';
    pub const WALLPAPER: char = '\u{e1bc}';
    pub const FILE_OPEN: char = '\u{eaf3}';
    pub const SAVE: char = '\u{e161}';
    pub const SYNC: char = '\u{e627}';
    pub const WARNING: char = '\u{f083}';
    pub const LOCK: char = '\u{e899}';
    pub const KEY: char = '\u{e73c}';
    pub const DEVICES: char = '\u{e326}';
    pub const DISPLAY_SETTINGS: char = '\u{eb97}';
    pub const TOUCH_APP: char = '\u{e913}';
    pub const GAMEPAD: char = '\u{e30f}';
    pub const SWAP_HORIZ: char = '\u{e8d4}';
    pub const LAYERS: char = '\u{e53b}';
    pub const IMAGE: char = '\u{e3f4}';
    pub const BRUSH: char = '\u{e3ae}';
    pub const GRID_VIEW: char = '\u{e9b0}';
    pub const RADIO_BUTTON_CHECKED: char = '\u{e837}';
    pub const RADIO_BUTTON_UNCHECKED: char = '\u{e836}';
    pub const CHECK_BOX: char = '\u{e9de}';
    pub const CHECK_BOX_OUTLINE_BLANK: char = '\u{e835}';
    pub const TIMER: char = '\u{e425}';
    pub const STOP: char = '\u{e047}';
    pub const FIBER_MANUAL_RECORD: char = '\u{e061}';
}

/// A button's emphasis (Material 3's common buttons).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Filled,
    Tonal,
    Outlined,
    Text,
    /// An extended floating action button (primary container, large shape).
    Fab,
}

/// What a component looks like under the pointer or the controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub hovered: bool,
    pub focused: bool,
    pub pressed: bool,
    pub disabled: bool,
}
impl State {
    /// The state layer's opacity.
    pub fn layer(self) -> f32 {
        if self.disabled {
            0.0
        } else if self.pressed {
            PRESSED
        } else if self.focused {
            FOCUS
        } else if self.hovered {
            HOVER
        } else {
            0.0
        }
    }
}

impl Ui<'_> {
    /// Text in a type scale step, its line box's top at `y`; returns its width.
    pub fn type_text(&mut self, ty: Type, x: f32, y: f32, color: u32, text: &str) -> f32 {
        self.text_line(ty.face, ty.size, ty.line, x, y, color, text)
    }

    /// `text` in `ty`, cut with an ellipsis to `width`.
    pub fn type_fit(&self, ty: Type, text: &str, width: f32) -> String {
        self.fit_text(ty.face, ty.size, text, width)
    }

    /// The advance of `text` in `ty`.
    pub fn type_width(&self, ty: Type, text: &str) -> f32 {
        self.measure(ty.face, ty.size, text)
    }

    /// An icon `size` dp square with its top-left at (`x`, `y`).
    pub fn icon(&mut self, icon: char, x: f32, y: f32, size: f32, color: u32, filled: bool) {
        let face = if filled { Face::IconFilled } else { Face::Icon };
        let mut buffer = [0; 4];
        self.text_line(face, size, size, x, y, color, icon.encode_utf8(&mut buffer));
    }

    /// The state layer over a component's shape.
    pub fn state_layer(&mut self, rect: [f32; 4], radius: f32, content: u32, state: State) {
        let opacity = state.layer();
        if opacity > 0.0 {
            let [x, y, w, h] = rect;
            self.rounded(x, y, w, h, radius, alpha(content, opacity));
        }
    }

    /// The focus indicator: a 3 dp ring in `secondary` just outside the shape.
    pub fn focus_ring(&mut self, rect: [f32; 4], radius: f32) {
        let [x, y, w, h] = rect;
        let color = scheme().secondary;
        let px = 1.0 / self.density;
        let rings = (3.0 * self.density).round().max(2.0) as i32;
        for i in 0..rings {
            let o = 2.0 + i as f32 * px;
            self.border(x - o, y - o, w + 2.0 * o, h + 2.0 * o, radius + o, color);
        }
    }

    /// A common button or an extended FAB in `rect`, with an optional leading icon; the label
    /// shrinks to fit narrow buttons.
    pub fn m3_button(
        &mut self,
        rect: [f32; 4],
        kind: Button,
        icon: Option<char>,
        label: &str,
        state: State,
    ) {
        let s = scheme();
        let [x, y, w, h] = rect;
        let radius = if kind == Button::Fab {
            SHAPE_LARGE
        } else {
            h / 2.0
        };
        let (container, content) = match kind {
            Button::Filled => (s.primary, s.on_primary),
            Button::Tonal => (s.secondary_container, s.on_secondary_container),
            Button::Outlined | Button::Text => (0, s.primary),
            Button::Fab => (s.primary_container, s.on_primary_container),
        };
        let (container, content) = if state.disabled {
            (
                if container == 0 {
                    0
                } else {
                    alpha(s.on_surface, DISABLED_CONTAINER)
                },
                alpha(s.on_surface, DISABLED_CONTENT),
            )
        } else {
            (container, content)
        };
        if container != 0 {
            self.rounded(x, y, w, h, radius, container);
        }
        if kind == Button::Outlined {
            let outline = if state.disabled {
                alpha(s.on_surface, DISABLED_CONTAINER)
            } else if state.focused {
                s.primary
            } else {
                s.outline
            };
            self.border(x, y, w, h, radius, outline);
        }
        self.state_layer(rect, radius, content, state);
        let ty = if kind == Button::Fab {
            Type {
                size: 16.0,
                line: 24.0,
                face: Face::Medium,
            }
        } else {
            LABEL_LARGE
        };
        let icon_size = if kind == Button::Fab { 24.0 } else { 18.0 };
        let padding: f32 = if kind == Button::Text {
            12.0
        } else if kind == Button::Fab {
            20.0
        } else {
            24.0
        };
        let padding = padding.min(w * 0.12).max(6.0);
        let lead = icon.map_or(0.0, |_| icon_size + 8.0);
        let room = (w - 2.0 * padding - lead).max(1.0);
        let natural = self.type_width(ty, label);
        let ty = if natural > room && !label.is_empty() {
            Type {
                size: (ty.size * room / natural).max(10.0),
                ..ty
            }
        } else {
            ty
        };
        let text = self.type_fit(ty, label, room);
        let text_w = self.type_width(ty, &text);
        let content_w = lead + text_w;
        let left = x + ((w - content_w) / 2.0).max(padding);
        if let Some(icon) = icon {
            self.icon(
                icon,
                left,
                y + (h - icon_size) / 2.0,
                icon_size,
                content,
                false,
            );
        }
        self.type_text(ty, left + lead, y + (h - ty.line) / 2.0, content, &text);
    }

    /// A switch with its top-left at (`x`, `y`): 52 by 32.
    pub fn m3_switch(&mut self, x: f32, y: f32, on: bool, state: State) {
        let s = scheme();
        let (w, h) = (52.0, 32.0);
        if on {
            self.rounded(x, y, w, h, h / 2.0, s.primary);
            let d = if state.pressed { 28.0 } else { 24.0 };
            let (cx, cy) = (x + w - 16.0, y + h / 2.0);
            self.rounded(cx - d / 2.0, cy - d / 2.0, d, d, d / 2.0, s.on_primary);
            self.icon(
                icon::CHECK,
                cx - 8.0,
                cy - 8.0,
                16.0,
                s.on_primary_container,
                false,
            );
        } else {
            self.rounded(x, y, w, h, h / 2.0, s.surface_container_highest);
            for i in 0..(2.0 * self.density).round().max(1.0) as i32 {
                let o = i as f32 / self.density;
                self.border(
                    x + o,
                    y + o,
                    w - 2.0 * o,
                    h - 2.0 * o,
                    h / 2.0 - o,
                    s.outline,
                );
            }
            let d = if state.pressed { 28.0 } else { 16.0 };
            let (cx, cy) = (x + 16.0, y + h / 2.0);
            self.rounded(cx - d / 2.0, cy - d / 2.0, d, d, d / 2.0, s.outline);
        }
        if state.layer() > 0.0 {
            let (cx, cy) = (if on { x + w - 16.0 } else { x + 16.0 }, y + h / 2.0);
            let layer = if on { s.primary } else { s.on_surface };
            self.rounded(
                cx - 20.0,
                cy - 20.0,
                40.0,
                40.0,
                20.0,
                alpha(layer, state.layer()),
            );
        }
    }

    /// A linear progress indicator: the active part in `primary`, a gap, then the track.
    pub fn m3_progress(&mut self, x: f32, y: f32, w: f32, fraction: f32) {
        let s = scheme();
        let h = 4.0;
        let done = (w * fraction.clamp(0.0, 1.0)).round();
        if done > 0.0 {
            self.rounded(x, y, done, h, h / 2.0, s.primary);
        }
        let gap = if done > 0.0 { 4.0 } else { 0.0 };
        if done + gap < w {
            self.rounded(
                x + done + gap,
                y,
                w - done - gap,
                h,
                h / 2.0,
                s.secondary_container,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_colours_swap_red_and_blue() {
        // Android's 0xAARRGGBB; the HUD's rgba() is 0xAABBGGRR.
        assert_eq!(
            argb(0x80_11_22_33),
            crate::hud::rgba(0x11, 0x22, 0x33, 0x80)
        );
    }

    #[test]
    fn missing_platform_roles_keep_the_baseline() {
        let mut colors = vec![0; ROLES.len()];
        colors[0] = 0xFF00_FF00;
        let scheme = Scheme::from_roles(&colors[..20]);
        assert_eq!(scheme.primary, crate::hud::rgba(0, 255, 0, 255));
        assert_eq!(scheme.on_primary, Scheme::BASELINE.on_primary);
        assert_eq!(scheme.outline, Scheme::BASELINE.outline);
    }

    #[test]
    fn a_state_layer_flattens_between_its_colours() {
        let black = crate::hud::rgba(0, 0, 0, 255);
        let white = crate::hud::rgba(255, 255, 255, 255);
        assert_eq!(over(black, white, 0.0), black);
        assert_eq!(over(black, white, 1.0), white);
        assert_eq!(
            over(black, white, 0.5),
            crate::hud::rgba(128, 128, 128, 255)
        );
    }
}

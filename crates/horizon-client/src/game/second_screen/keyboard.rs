//! The panel's keyboard of vanilla buttons, shared by the Chat tab and the title menus' text
//! fields: two layers (letters and symbols), Shift (twice locks capitals), and Delete that
//! repeats while held.
use horizon_client::hud::{HudAtlas, HudBatch};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Key {
    Char(char),
    Shift,
    Layer,
    Space,
    Delete,
    /// The previous message sent (the Up arrow).
    History,
    Send,
}

/// What a key typed.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Typed {
    Text(String),
    Delete,
    History,
    Send,
}

/// Which keys the bottom row has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Layout {
    /// The chat: `/` for commands, ↑ for the history, Send.
    Chat,
    /// A menu's text field (a world's name or seed, a server's address): Done.
    Field,
}

/// A row of keys and their widths in key units (at most 10, centred).
type Row = &'static [(Key, f32)];

const fn c(ch: char) -> (Key, f32) {
    (Key::Char(ch), 1.0)
}

const fn keys<const N: usize>(chars: [char; N]) -> [(Key, f32); N] {
    let mut out = [(Key::Space, 1.0); N];
    let mut i = 0;
    while i < N {
        out[i] = c(chars[i]);
        i += 1;
    }
    out
}

const DIGITS: [(Key, f32); 10] = keys(['1', '2', '3', '4', '5', '6', '7', '8', '9', '0']);
const TOP: [(Key, f32); 10] = keys(['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p']);
const MIDDLE: [(Key, f32); 9] = keys(['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l']);
const LOWER: [(Key, f32); 9] = [
    (Key::Shift, 1.5),
    c('z'),
    c('x'),
    c('c'),
    c('v'),
    c('b'),
    c('n'),
    c('m'),
    (Key::Delete, 1.5),
];
/// The slash starts a command, so it is on the bottom row of both layers.
const CHAT_BOTTOM: [(Key, f32); 6] = [
    (Key::Layer, 1.5),
    c('/'),
    (Key::Space, 4.0),
    c('.'),
    (Key::History, 1.0),
    (Key::Send, 1.5),
];
const FIELD_BOTTOM: [(Key, f32); 6] = [
    (Key::Layer, 1.5),
    c('-'),
    (Key::Space, 4.0),
    c('.'),
    c('_'),
    (Key::Send, 1.5),
];

const SYMBOLS_1: [(Key, f32); 10] = keys(['!', '@', '#', '$', '%', '^', '&', '*', '(', ')']);
const SYMBOLS_2: [(Key, f32); 10] = keys(['-', '_', '=', '+', '[', ']', '{', '}', '\\', '|']);
const SYMBOLS_3: [(Key, f32); 9] = keys([';', ':', '\'', '"', ',', '<', '>', '?', '~']);
const SYMBOLS_4: [(Key, f32); 9] = [
    (Key::Shift, 1.5),
    c('`'),
    c('§'),
    c('°'),
    c('é'),
    c('è'),
    c('à'),
    c('ñ'),
    (Key::Delete, 1.5),
];

pub(super) const GAP: f32 = 2.0;
const KEY_H: f32 = 18.0;
/// The keyboard's height at its smallest keys (the Chat tab's).
pub(super) const HEIGHT: f32 = 5.0 * KEY_H + 4.0 * GAP;
/// Delete held repeats after this, this often.
const REPEAT_AFTER: Duration = Duration::from_millis(400);
const REPEAT_EVERY: Duration = Duration::from_millis(50);
/// A tapped key stays lit this long.
const FLASH: Duration = Duration::from_millis(120);

#[derive(Default)]
struct State {
    symbols: bool,
    /// The next letter is upper case (Shift), or every letter (Shift twice).
    shift: bool,
    caps: bool,
    /// Delete held: since, and its last repeat.
    held: Option<(Instant, Instant)>,
    /// The key last tapped (row, column), lit for a moment.
    flash: Option<(usize, usize, Instant)>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn rows(symbols: bool, layout: Layout) -> [Row; 5] {
    let bottom: Row = match layout {
        Layout::Chat => &CHAT_BOTTOM,
        Layout::Field => &FIELD_BOTTOM,
    };
    if symbols {
        [&SYMBOLS_1, &SYMBOLS_2, &SYMBOLS_3, &SYMBOLS_4, bottom]
    } else {
        [&DIGITS, &TOP, &MIDDLE, &LOWER, bottom]
    }
}

/// Every key's row, column and rectangle in `area`, its five rows filling its height.
fn key_rects(area: [f32; 4], symbols: bool, layout: Layout) -> Vec<(usize, usize, Key, [f32; 4])> {
    let [x, y, w, h] = area;
    let unit = (w + GAP) / 10.0;
    let key_h = ((h - 4.0 * GAP) / 5.0).floor().max(KEY_H);
    let mut out = Vec::new();
    for (r, row) in rows(symbols, layout).iter().enumerate() {
        let total: f32 = row.iter().map(|k| k.1).sum();
        let mut left = x + (10.0 - total) / 2.0 * unit;
        let ky = y + r as f32 * (key_h + GAP);
        for (col, (key, units)) in row.iter().enumerate() {
            let kx = left.round();
            let kw = (left + units * unit - GAP).round() - kx;
            out.push((r, col, *key, [kx, ky, kw, key_h]));
            left += units * unit;
        }
    }
    out
}

pub(super) fn inside([x, y, w, h]: [f32; 4], p: [f32; 2]) -> bool {
    (x..x + w).contains(&p[0]) && (y..y + h).contains(&p[1])
}

fn letter(state: &State, ch: char) -> String {
    if state.shift || state.caps {
        ch.to_uppercase().collect()
    } else {
        ch.to_string()
    }
}

/// A vanilla button with a centred label.
pub(super) fn button(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    rect: [f32; 4],
    label: &str,
    lit: bool,
) {
    let [x, y, w, h] = rect;
    let sprite = if lit {
        "gui/sprites/widget/button_highlighted"
    } else {
        "gui/sprites/widget/button"
    };
    horizon_client::ui::nine_slice(batch, atlas, sprite, x, y, w, h, u32::MAX);
    let tw = HudBatch::text_width(atlas, label);
    let color = if lit { 0xFFFF_FFA0 } else { u32::MAX };
    batch.text(
        atlas,
        label,
        (x + (w - tw) / 2.0).floor(),
        (y + (h - 8.0) / 2.0).floor(),
        color,
        true,
    );
}

/// The keyboard in `area`.
pub(super) fn draw(batch: &mut HudBatch, atlas: &HudAtlas, area: [f32; 4], layout: Layout) {
    STATE.with_borrow(|s| {
        for (r, col, key, rect) in key_rects(area, s.symbols, layout) {
            let flashed = s
                .flash
                .is_some_and(|(fr, fc, at)| (fr, fc) == (r, col) && at.elapsed() < FLASH);
            let lit = flashed
                || (key == Key::Shift && (s.shift || s.caps))
                || (key == Key::Layer && s.symbols);
            let label = match key {
                Key::Char(ch) => letter(s, ch),
                Key::Shift if s.caps => "CAPS".to_owned(),
                Key::Shift => "⇧".to_owned(),
                Key::Layer if s.symbols => "ABC".to_owned(),
                Key::Layer => "?123".to_owned(),
                Key::Space => "Space".to_owned(),
                Key::Delete => "⌫".to_owned(),
                Key::History => "↑".to_owned(),
                Key::Send if layout == Layout::Field => "Done".to_owned(),
                Key::Send => "Send".to_owned(),
            };
            button(batch, atlas, rect, &label, lit);
        }
    });
}

/// A touch starting at `point` on the keyboard in `area`: what it typed (Shift and the layer key
/// type nothing, they change the keys).
pub(super) fn press(area: [f32; 4], point: [f32; 2], layout: Layout) -> Option<Typed> {
    STATE.with_borrow_mut(|s| {
        let (r, col, key, _) = key_rects(area, s.symbols, layout)
            .into_iter()
            .find(|(_, _, _, rect)| inside(*rect, point))?;
        s.flash = Some((r, col, Instant::now()));
        match key {
            Key::Shift => {
                // Shift, then Shift again locks capitals; a third press lets go.
                (s.shift, s.caps) = match (s.shift, s.caps) {
                    (false, false) => (true, false),
                    (true, false) => (false, true),
                    _ => (false, false),
                };
                None
            }
            Key::Layer => {
                s.symbols = !s.symbols;
                None
            }
            Key::Char(ch) => {
                let text = letter(s, ch);
                s.shift = false;
                Some(Typed::Text(text))
            }
            Key::Space => Some(Typed::Text(" ".to_owned())),
            Key::Delete => {
                let now = Instant::now();
                s.held = Some((now, now));
                Some(Typed::Delete)
            }
            Key::History => Some(Typed::History),
            Key::Send => Some(Typed::Send),
        }
    })
}

/// The touch let go: Delete stops repeating.
pub(super) fn release() {
    STATE.with_borrow_mut(|s| s.held = None);
}

/// Once a frame: whether Delete, held, repeats now.
pub(super) fn repeat() -> bool {
    STATE.with_borrow_mut(|s| {
        let Some((since, last)) = s.held.as_mut() else {
            return false;
        };
        let due = since.elapsed() >= REPEAT_AFTER && last.elapsed() >= REPEAT_EVERY;
        if due {
            *last = Instant::now();
        }
        due
    })
}

/// A key is lit: the panel redraws until it goes out.
pub(super) fn flashing() -> bool {
    STATE.with_borrow(|s| s.flash.is_some_and(|(_, _, at)| at.elapsed() < FLASH))
}

//! The Chat tab: the chat history over the vanilla chat screen's input (its argument colours,
//! usage lines, history and clickable messages) in a box at a smaller GUI scale, the
//! completions at the cursor as buttons to tap, and a keyboard of vanilla buttons. A key opens
//! the chat screen if it is closed; Send sends and closes it, as Enter does.
use super::{
    super::{Game, Screen, chat::CHAT_LIFT, event_loop::EventLoopCtl},
    tabs::{self, Tab},
};
use crate::second_screen::{Phase, Touch};
use horizon_client::hud::{HudBatch, rgba};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
    window::WindowId,
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
const BOTTOM: [(Key, f32); 6] = [
    (Key::Layer, 1.5),
    c('/'),
    (Key::Space, 4.0),
    c('.'),
    (Key::History, 1.0),
    (Key::Send, 1.5),
];
const LETTERS: [Row; 5] = [&DIGITS, &TOP, &MIDDLE, &LOWER, &BOTTOM];

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
const SYMBOLS: [Row; 5] = [&SYMBOLS_1, &SYMBOLS_2, &SYMBOLS_3, &SYMBOLS_4, &BOTTOM];

const KEY_H: f32 = 18.0;
const GAP: f32 = 2.0;
const CHIP_H: f32 = 16.0;
/// The paging buttons beside the completions.
const PAGE_W: f32 = 14.0;
const PADDING: f32 = 4.0;
/// Delete held repeats after this, this often.
const REPEAT_AFTER: Duration = Duration::from_millis(400);
const REPEAT_EVERY: Duration = Duration::from_millis(50);
/// A tapped key stays lit this long.
const FLASH: Duration = Duration::from_millis(120);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Chip {
    Complete(usize),
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Pointer {
    /// On the chat's box: the chat screen's own pointer.
    Box,
    /// Anywhere else on the tab: only the press counts.
    Elsewhere,
}

#[derive(Default)]
struct State {
    symbols: bool,
    /// The next letter is upper case (Shift), or every letter (Shift twice).
    shift: bool,
    caps: bool,
    /// The first completion shown, and the list it is of (a new list starts again).
    chips_from: usize,
    chips_of: Vec<String>,
    /// The completion buttons as last drawn.
    chips: Vec<([f32; 4], Chip)>,
    /// The tab's panel as last drawn.
    panel: Option<[f32; 4]>,
    /// The chat box's place and scale on the panel as last drawn: x, y, and panel pixels per
    /// box pixel.
    chat_at: Option<([f32; 2], f32)>,
    pointer: Option<Pointer>,
    /// Delete held: since, and its last repeat.
    held: Option<(Instant, Instant)>,
    /// The key last tapped (row, column), lit for a moment.
    flash: Option<(usize, usize, Instant)>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Where the tab's parts go in its panel: the chat's box, the completions and the keyboard.
struct Areas {
    chat: [f32; 4],
    chips: [f32; 4],
    keys: [f32; 4],
}

fn areas([px, py, pw, ph]: [f32; 4]) -> Areas {
    let (x, y, w, h) = (
        px + PADDING,
        py + PADDING,
        pw - 2.0 * PADDING,
        ph - 2.0 * PADDING,
    );
    let keys_h = 5.0 * KEY_H + 4.0 * GAP;
    let keys = [x, y + h - keys_h, w, keys_h];
    let chips = [x, keys[1] - GAP - CHIP_H, w, CHIP_H];
    let chat = [x, y, w, (chips[1] - GAP - y).max(0.0)];
    Areas { chat, chips, keys }
}

/// The chat's box GUI scale for a panel at `scale`: smaller, so the chat's 320-pixel lines fit.
fn box_scale(scale: f32) -> f32 {
    ((scale * 3.0 / 4.0).floor()).max(1.0)
}

/// The chat box's size in its own GUI pixels, for a panel `panel` at `scale`.
fn box_size(panel: [f32; 4], scale: f32) -> [f32; 2] {
    let k = box_scale(scale) / scale;
    let [_, _, w, h] = areas(panel).chat;
    [(w / k).floor(), (h / k).floor()]
}

/// The chat screen's box (its GUI width and height) on a panel `size` pixels big.
pub(super) fn chat_box(size: [u32; 2]) -> [f32; 2] {
    let (panel, scale) = super::tab_panel(size);
    box_size(panel, scale as f32)
}

fn rows(symbols: bool) -> &'static [Row; 5] {
    if symbols { &SYMBOLS } else { &LETTERS }
}

/// Every key's row, column and rectangle in `area`.
fn key_rects(area: [f32; 4], symbols: bool) -> Vec<(usize, usize, Key, [f32; 4])> {
    let [x, y, w, _] = area;
    let unit = (w + GAP) / 10.0;
    let mut out = Vec::new();
    for (r, row) in rows(symbols).iter().enumerate() {
        let total: f32 = row.iter().map(|k| k.1).sum();
        let mut left = x + (10.0 - total) / 2.0 * unit;
        let ky = y + r as f32 * (KEY_H + GAP);
        for (col, (key, units)) in row.iter().enumerate() {
            let kx = left.round();
            let kw = (left + units * unit - GAP).round() - kx;
            out.push((r, col, *key, [kx, ky, kw, KEY_H]));
            left += units * unit;
        }
    }
    out
}

fn inside([x, y, w, h]: [f32; 4], p: [f32; 2]) -> bool {
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
fn button(batch: &mut HudBatch, game: &Game<'_>, rect: [f32; 4], label: &str, lit: bool) {
    let [x, y, w, h] = rect;
    let atlas = &game.gui.atlas;
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

/// `text` cut to `width` GUI pixels, with an ellipsis when it was longer.
fn fit(game: &Game<'_>, text: &str, width: f32) -> String {
    let atlas = &game.gui.atlas;
    if HudBatch::text_width(atlas, text) <= width {
        return text.to_owned();
    }
    let mut out = String::new();
    for ch in text.chars() {
        out.push(ch);
        if HudBatch::text_width(atlas, &format!("{out}…")) > width {
            out.pop();
            break;
        }
    }
    out.push('…');
    out
}

impl Game<'_> {
    /// The Chat tab in its `panel`.
    pub(in crate::game) fn draw_chat_tab(&mut self, batch: &mut HudBatch, panel: [f32; 4]) {
        let areas = areas(panel);
        let atlas = &self.gui.atlas;
        // The chat's box: drawn at its own scale, clipped to it, and scaled onto the panel.
        let [cx, cy, cw, ch] = areas.chat;
        batch.fill(atlas, cx, cy, cw, ch, rgba(0, 0, 0, 200));
        let scale = box_scale(batch.scale);
        let k = scale / batch.scale;
        let [bw, bh] = box_size(panel, batch.scale);
        let mut chat = HudBatch::new([1, 1]);
        chat.scale = scale;
        chat.width = bw;
        chat.height = bh;
        chat.game_time = batch.game_time;
        // The history right over the input (`CHAT_LIFT`), all of it as the open chat shows it.
        chat.height = bh + CHAT_LIFT;
        self.ui
            .chat
            .draw(&mut chat, &self.gui.atlas, self.ticks, true);
        chat.height = bh;
        if self.chat_hosted() {
            self.draw_chat_screen(&mut chat);
        } else {
            // The closed input: tap it, or a key, to type.
            let atlas = &self.gui.atlas;
            chat.fill(atlas, 2.0, bh - 14.0, bw - 4.0, 12.0, rgba(0, 0, 0, 128));
            chat.text(atlas, "Tap to chat", 4.0, bh - 12.0, 0xFF80_8080, true);
        }
        chat.clip_since(0, 0.0, 0.0, bw, bh);
        batch.append_scaled(chat, k, cx, cy);
        // The completions at the cursor, a page of buttons.
        let list = self.chat_completions();
        let mut chips = Vec::new();
        let from = STATE.with_borrow_mut(|s| {
            if s.chips_of != list {
                s.chips_of.clone_from(&list);
                s.chips_from = 0;
            }
            s.chips_from.min(list.len())
        });
        let [x0, y0, width, _] = areas.chips;
        let mut x = x0;
        if from > 0 {
            chips.push(([x, y0, PAGE_W, CHIP_H], Chip::Previous));
            x += PAGE_W + GAP;
        }
        for (i, text) in list.iter().enumerate().skip(from) {
            let label = fit(self, text, width - 2.0 * (PAGE_W + GAP) - 8.0);
            let w = (HudBatch::text_width(&self.gui.atlas, &label) + 8.0).max(CHIP_H);
            let room = x0 + width
                - if i + 1 < list.len() {
                    PAGE_W + GAP
                } else {
                    0.0
                };
            if x + w > room {
                chips.push(([x0 + width - PAGE_W, y0, PAGE_W, CHIP_H], Chip::Next));
                break;
            }
            chips.push(([x, y0, w, CHIP_H], Chip::Complete(i)));
            x += w + GAP;
        }
        for (rect, chip) in &chips {
            let label = match chip {
                Chip::Complete(i) => fit(self, &list[*i], rect[2] - 8.0),
                Chip::Previous => "<".to_owned(),
                Chip::Next => ">".to_owned(),
            };
            button(batch, self, *rect, &label, false);
        }
        // The keyboard.
        let (symbols, shift, caps, flash) =
            STATE.with_borrow(|s| (s.symbols, s.shift, s.caps, s.flash));
        let state = STATE.with_borrow(|s| State {
            shift: s.shift,
            caps: s.caps,
            ..State::default()
        });
        for (r, col, key, rect) in key_rects(areas.keys, symbols) {
            let flashed =
                flash.is_some_and(|(fr, fc, at)| (fr, fc) == (r, col) && at.elapsed() < FLASH);
            let lit =
                flashed || (key == Key::Shift && (shift || caps)) || (key == Key::Layer && symbols);
            let label = match key {
                Key::Char(ch) => letter(&state, ch),
                Key::Shift if caps => "CAPS".to_owned(),
                Key::Shift => "⇧".to_owned(),
                Key::Layer if symbols => "ABC".to_owned(),
                Key::Layer => "?123".to_owned(),
                Key::Space => "Space".to_owned(),
                Key::Delete => "⌫".to_owned(),
                Key::History => "↑".to_owned(),
                Key::Send => "Send".to_owned(),
            };
            button(batch, self, rect, &label, lit);
        }
        STATE.with_borrow_mut(|s| {
            s.chips = chips;
            s.panel = Some(panel);
            s.chat_at = Some(([cx, cy], k));
        });
    }

    /// Once a frame: Delete held repeats.
    pub(in crate::game) fn chat_keyboard_tick(&mut self) {
        let repeat = STATE.with_borrow_mut(|s| {
            let (since, last) = s.held.as_mut()?;
            (since.elapsed() >= REPEAT_AFTER && last.elapsed() >= REPEAT_EVERY).then(|| {
                *last = Instant::now();
            })
        });
        if repeat.is_some() && self.screen == Screen::Chat {
            self.chat_key_code(KeyCode::Backspace, None);
        }
    }

    /// A key on the panel's keyboard.
    fn chat_key_tap(&mut self, key: Key) {
        match key {
            Key::Shift => {
                // Shift, then Shift again locks capitals; a third press lets go.
                STATE.with_borrow_mut(|s| {
                    (s.shift, s.caps) = match (s.shift, s.caps) {
                        (false, false) => (true, false),
                        (true, false) => (false, true),
                        _ => (false, false),
                    };
                });
                return;
            }
            Key::Layer => {
                STATE.with_borrow_mut(|s| s.symbols = !s.symbols);
                return;
            }
            _ => {}
        }
        // Typing opens the chat, as the chat key does.
        if self.screen == Screen::Playing {
            self.open_chat_method("");
        }
        if self.screen != Screen::Chat {
            return;
        }
        match key {
            Key::Char(ch) => {
                let text = STATE.with_borrow_mut(|s| {
                    let text = letter(s, ch);
                    s.shift = false;
                    text
                });
                self.chat_type(&text);
            }
            Key::Space => self.chat_type(" "),
            Key::Delete => self.chat_key_code(KeyCode::Backspace, None),
            Key::History => self.chat_key_code(KeyCode::ArrowUp, None),
            Key::Send => self.chat_key_code(KeyCode::Enter, None),
            Key::Shift | Key::Layer => {}
        }
    }
}

/// A touch at `point` (panel GUI pixels) on the Chat tab's panel: false when it isn't one.
pub(super) fn touch(
    game: &mut Game<'_>,
    touch: &Touch,
    point: [f32; 2],
    events: &dyn EventLoopCtl,
    id: WindowId,
) -> bool {
    let ended = matches!(touch.phase, Phase::Ended | Phase::Cancelled);
    let pointer = if touch.phase == Phase::Started {
        let Some(panel) = STATE.with_borrow(|s| s.panel) else {
            return false;
        };
        if tabs::selected() != Tab::Chat || !inside(panel, point) || tabs::on_tabs(point) {
            return false;
        }
        let pointer = press(game, panel, point);
        STATE.with_borrow_mut(|s| s.pointer = Some(pointer));
        pointer
    } else {
        let Some(pointer) = STATE.with_borrow(|s| s.pointer) else {
            return false;
        };
        pointer
    };
    if ended {
        STATE.with_borrow_mut(|s| {
            s.pointer = None;
            s.held = None;
        });
    }
    // The chat's box: the chat screen's pointer, pressed, moved and released.
    if pointer == Pointer::Box && game.chat_hosted() {
        let Some(([x, y], k)) = STATE.with_borrow(|s| s.chat_at) else {
            return true;
        };
        let Some(main) = game.gui_scale() else {
            return true;
        };
        let at = [(point[0] - x) / k, (point[1] - y) / k];
        let device_id = DeviceId::dummy();
        game.handle_window_event(
            events,
            id,
            WindowEvent::CursorMoved {
                device_id,
                position: PhysicalPosition::new(
                    f64::from(at[0] * main.scale),
                    f64::from(at[1] * main.scale),
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
    }
    true
}

/// A touch starting at `point` on the tab's `panel`: a completion, a key, or the chat's box.
fn press(game: &mut Game<'_>, panel: [f32; 4], point: [f32; 2]) -> Pointer {
    let areas = areas(panel);
    if inside(areas.chat, point) {
        if !game.chat_hosted() {
            if game.screen == Screen::Playing {
                game.open_chat_method("");
            }
            return Pointer::Elsewhere;
        }
        return Pointer::Box;
    }
    let chip = STATE.with_borrow(|s| {
        s.chips
            .iter()
            .find(|(rect, _)| inside(*rect, point))
            .map(|(_, chip)| *chip)
    });
    if let Some(chip) = chip {
        match chip {
            Chip::Complete(i) => {
                game.chat_complete(i);
            }
            Chip::Previous | Chip::Next => STATE.with_borrow_mut(|s| {
                let shown = s
                    .chips
                    .iter()
                    .filter(|(_, c)| matches!(c, Chip::Complete(_)))
                    .count()
                    .max(1);
                s.chips_from = if chip == Chip::Next {
                    s.chips_from + shown
                } else {
                    s.chips_from.saturating_sub(shown)
                };
            }),
        }
        return Pointer::Elsewhere;
    }
    let symbols = STATE.with_borrow(|s| s.symbols);
    if let Some((r, col, key, _)) = key_rects(areas.keys, symbols)
        .into_iter()
        .find(|(_, _, _, rect)| inside(*rect, point))
    {
        STATE.with_borrow_mut(|s| {
            s.flash = Some((r, col, Instant::now()));
            if key == Key::Delete {
                let now = Instant::now();
                s.held = Some((now, now));
            }
        });
        game.chat_key_tap(key);
    }
    Pointer::Elsewhere
}

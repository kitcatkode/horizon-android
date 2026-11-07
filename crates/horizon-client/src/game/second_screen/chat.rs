//! The Chat tab: the chat history over the vanilla chat screen's input (its argument colours,
//! usage lines, history and clickable messages) in a box at a smaller GUI scale, the
//! completions at the cursor as buttons to tap, and a keyboard of vanilla buttons. A key opens
//! the chat screen if it is closed; Send sends and closes it, as Enter does.
use super::{
    super::{Game, Screen, chat::CHAT_LIFT, event_loop::EventLoopCtl},
    keyboard::{self, GAP, Layout, Typed, inside},
    tabs::{self, Tab},
};
use crate::second_screen::{Phase, Touch};
use horizon_client::hud::{HudBatch, rgba};
use std::cell::RefCell;
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode,
    window::WindowId,
};

const CHIP_H: f32 = 16.0;
/// The paging buttons beside the completions.
const PAGE_W: f32 = 14.0;
const PADDING: f32 = 4.0;

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
    let keys = [x, y + h - keyboard::HEIGHT, w, keyboard::HEIGHT];
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
            keyboard::button(batch, &self.gui.atlas, *rect, &label, false);
        }
        keyboard::draw(batch, &self.gui.atlas, areas.keys, Layout::Chat);
        STATE.with_borrow_mut(|s| {
            s.chips = chips;
            s.panel = Some(panel);
            s.chat_at = Some(([cx, cy], k));
        });
    }

    /// Once a frame: Delete held repeats.
    pub(in crate::game) fn chat_keyboard_tick(&mut self) {
        if keyboard::repeat() && self.screen == Screen::Chat {
            self.chat_key_code(KeyCode::Backspace, None);
        }
    }

    /// What a key on the panel's keyboard typed.
    fn chat_key_tap(&mut self, typed: Typed) {
        // Typing opens the chat, as the chat key does.
        if self.screen == Screen::Playing {
            self.open_chat_method("");
        }
        if self.screen != Screen::Chat {
            return;
        }
        match typed {
            Typed::Text(text) => self.chat_type(&text),
            Typed::Delete => self.chat_key_code(KeyCode::Backspace, None),
            Typed::History => self.chat_key_code(KeyCode::ArrowUp, None),
            Typed::Send => self.chat_key_code(KeyCode::Enter, None),
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
        STATE.with_borrow_mut(|s| s.pointer = None);
        keyboard::release();
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
    if let Some(typed) = keyboard::press(areas.keys, point, Layout::Chat) {
        game.chat_key_tap(typed);
    }
    Pointer::Elsewhere
}

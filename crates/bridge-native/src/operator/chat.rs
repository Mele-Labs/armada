//! The chat with the Operator, which Ask opens about one item inside its card, and which the pane
//! from a Job holds. It holds conversation only: what the Operator raises is an item of Your turn, never a
//! line here. The page sends the words (`talk_to_operator`); this draws them and the box.

use gpui::{div, prelude::*, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, KeyDownEvent, Role, SharedString, Window};
use gpui_component::input::{Input, InputEvent, InputState};
use ipc::operator_talk::OperatorTalkLine;

use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, kbd, spinner};

/// What the chat asks of the page.
#[derive(Clone, Debug, PartialEq)]
pub enum ChatAsked {
    /// The words typed, to send about the chat's item.
    Send(String),
    /// Esc in the box: back to the page.
    Leave,
}

impl EventEmitter<ChatAsked> for OperatorChat {}

pub struct OperatorChat {
    pub input: Entity<InputState>,
    pub lines: Vec<OperatorTalkLine>,
    /// `turn:<n>`, the item asked about; the pane's chat is about nothing.
    pub about: Option<String>,
    /// That item's title, as its card says it.
    pub about_title: Option<SharedString>,
    /// Words sent, and the Operator's answer not in yet.
    pub waiting: Option<String>,
    pub refused: Option<SharedString>,
    /// Fleet has answered the conversation so far.
    pub read: bool,
    _typed: gpui::Subscription,
}

impl OperatorChat {
    pub fn new(about: Option<String>, about_title: Option<SharedString>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Ask the Operator"));
        let typed = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { secondary: false, .. } = event {
                let words = input.read(cx).value().trim().to_string();
                if words.is_empty() || this.waiting.is_some() {
                    return;
                }
                input.update(cx, |input, cx| input.set_value("", window, cx));
                this.waiting = Some(words.clone());
                this.refused = None;
                cx.emit(ChatAsked::Send(words));
                cx.notify();
            }
        });
        OperatorChat { input, lines: Vec::new(), about, about_title, waiting: None, refused: None, read: false, _typed: typed }
    }

    pub fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }

    /// The conversation so far, as Fleet keeps it.
    pub fn set_lines(&mut self, lines: Vec<OperatorTalkLine>, cx: &mut Context<Self>) {
        self.lines = lines;
        self.read = true;
        cx.notify();
    }

    /// The Operator answered the words that were waiting.
    pub fn answered(&mut self, said: String, at: String, cx: &mut Context<Self>) {
        if let Some(words) = self.waiting.take() {
            self.lines.push(OperatorTalkLine { from_person: true, text: words, at: at.clone(), about: self.about.clone() });
        }
        self.lines.push(OperatorTalkLine { from_person: false, text: said, at, about: self.about.clone() });
        cx.notify();
    }

    /// The words did not reach the Operator: they go back in the box, with why.
    pub fn refused(&mut self, why: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(words) = self.waiting.take() {
            self.input.update(cx, |input, cx| input.set_value(words, window, cx));
        }
        self.refused = Some(why.into());
        cx.notify();
    }

    fn line(&self, at: usize, line: &OperatorTalkLine, look: &Look) -> impl IntoElement {
        let mine = line.from_person;
        let bubble = div()
            .id(SharedString::from(format!("talk-{at}")))
            .max_w(gpui::relative(0.86))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_LG))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .when(mine, |bubble| bubble.bg(look.color(token::BG_HOVER)))
            .when(!mine, |bubble| bubble.bg(look.color(token::OPERATOR_MUTED)))
            .child(line.text.clone());
        let name = if mine { format!("You: {}", line.text) } else { format!("Operator: {}", line.text) };
        div()
            .flex()
            .gap(look.length(token::SPACE_2))
            .items_start()
            .when(mine, |row| row.justify_end())
            .when(!mine, |row| row.child(div().pt(look.length(token::SPACE_2)).child(glyph("bot", look.length(token::SPACE_4), look.color(token::OPERATOR)))))
            .child(labelled(bubble, Role::ListItem, name))
    }
}

impl Render for OperatorChat {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let lines: Vec<_> = self.lines.iter().enumerate().map(|(at, line)| self.line(at, line, look).into_any_element()).collect();
        let waiting = self.waiting.clone().map(|words| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(self.line(usize::MAX, &OperatorTalkLine { from_person: true, text: words, at: String::new(), about: None }, look))
                .child(labelled(
                    div()
                        .id("talk-reading")
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(spinner("talk-spin", look.length(token::SPACE_4), look.color(token::OPERATOR), cx))
                        .child("Reading"),
                    Role::Status,
                    "The Operator is reading",
                ))
        });
        let refused = self.refused.clone().map(|why| {
            labelled(
                div().id("talk-refused").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()),
                Role::Alert,
                why,
            )
        });
        let about = self.about_title.clone().map(|title| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph("bot", look.length(token::SPACE_3), look.color(token::OPERATOR)))
                .child(div().flex_1().min_w_0().truncate().child(format!("About {title}")))
                .child(kbd("Esc", look))
        });
        let skeleton = (!self.read).then(|| {
            div().flex().flex_col().gap(look.length(token::SPACE_2)).children((0..3).map(|at| {
                div().h(look.length(token::SPACE_8)).w(gpui::relative(if at % 2 == 0 { 0.7 } else { 0.5 })).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_HOVER))
            }))
        });
        labelled(
            div()
                .id("operator-chat")
                .flex()
                .flex_col()
                .size_full()
                .gap(look.length(token::SPACE_3))
                // Before the box, which would take Esc for itself.
                .capture_key_down(cx.listener(|_, event: &KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        cx.emit(ChatAsked::Leave);
                    }
                }))
                .children(about)
                .child(
                    div()
                        .id("operator-chat-lines")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .justify_end()
                        .gap(look.length(token::SPACE_2))
                        .children(skeleton)
                        .children(lines)
                        .children(waiting),
                )
                .children(refused)
                .child(
                    labelled(
                        div()
                            .id("operator-chat-box")
                            .rounded(look.length(token::RADIUS_LG))
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::OPERATOR_EDGE))
                            .bg(look.color(token::BG_RAISED))
                            .font_weight(FontWeight(look.number(token::WEIGHT_BODY)))
                            .child(Input::new(&self.input).appearance(false).bordered(false)),
                        Role::TextInput,
                        "Ask the Operator",
                    ),
                ),
            Role::Region,
            "Operator chat",
        )
    }
}

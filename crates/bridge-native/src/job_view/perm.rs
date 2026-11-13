//! The permission card (`job-view.md`, B4.3 A): the call the Job is held on, and the answers to it.
//! A call that reaches a folder also offers what always applies to, and calls like this one.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{AnswerSessionAsk, CallAnswer, CallInFlight, CallKind, ReachesTo, SessionId, ThreadAsk};

use super::acts::{button, kbd, Tone};
use super::dock::first_line;
use super::rows::{eyebrow, glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a reach card's folder and kind pickers hold.
#[derive(Default, Clone)]
pub(super) struct Perm {
    pub folder: Option<usize>,
    pub wide: bool,
    pub only_in: bool,
}

/// The wording of an offer, and what choosing it means (`CALL_ANSWER`).
fn offer_words(answer: CallAnswer, reach: bool) -> (&'static str, &'static str) {
    match answer {
        CallAnswer::AllowOnce if reach => ("Allow once", "Runs it now. The next call outside its slots asks again."),
        CallAnswer::AllowOnce => ("Allow once", "Runs it now. Nothing is written down, and the same call asks again next time."),
        CallAnswer::AllowAndRemember => ("Allow and remember", "Runs it now, and the rule goes into this repository's own personal agent settings, so your terminal stops asking too."),
        CallAnswer::Refuse if reach => ("Reject once", "Not run. The job is told no, and the next call asks again."),
        CallAnswer::Refuse => ("Refuse", "The job is told no, and says what it could not do."),
        CallAnswer::AlwaysAllow => ("Always allow", "Runs it now. Every job reads and writes in the folder chosen, and under it, with no card. Settings, Features lists it."),
        CallAnswer::AlwaysBlock => ("Always block", "Not run. Every job is refused the folder chosen, and under it, with no card. Settings, Features lists it."),
        CallAnswer::AllowForSession => ("Always allow for this job", "Runs it now. This job reads and writes in the folder chosen, and under it, with no card, until it is closed. Settings does not list it."),
        CallAnswer::AllowLikeThisForSession => ("Allow calls like this for this job", "Runs it, and every call of this kind until the job is closed. Settings does not list it."),
    }
}

impl JobView {
    fn perm_of(&self, id: &str, call: &str) -> Perm {
        self.dock_of(id).and_then(|dock| dock.perms.get(call).cloned()).unwrap_or_default()
    }

    fn set_perm(&mut self, call: &str, change: impl FnOnce(&mut Perm), cx: &mut Context<Self>) {
        if let Some(dock) = self.dock_mut() {
            change(dock.perms.entry(call.to_string()).or_default());
        }
        cx.notify();
    }

    /// An answer to the call the Job is held on.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn permit(&mut self, id: &SharedString, call: &str, answer: CallAnswer, folder: Option<String>, kind: Option<CallKind>, window: &mut gpui::Window, cx: &mut Context<Self>) {
        let body = ipc::encode(&AnswerSessionAsk { session_id: SessionId::carried(id.to_string()), call: call.to_string(), answer, note: None, answers: Vec::new(), folder, kind }).map_err(|why| format!("{why:?}"));
        let key = format!("question:{call}");
        self.post_with(
            id.clone(),
            "/sessions/ask/answer",
            body,
            move |this, why, cx| {
                if let Some(dock) = this.dock_mut() {
                    dock.said.insert(key, why);
                }
                cx.notify();
            },
            window,
            cx,
        );
        self.aim(&format!("question:{call}"));
        cx.notify();
    }

    /// One answer as a button, with its meaning as a tooltip.
    #[allow(clippy::too_many_arguments)]
    fn offer_button(&self, id: &SharedString, call: &str, offer: CallAnswer, reach: bool, hint: Option<&'static str>, extra: Option<(String, Option<CallKind>, bool)>, folder_name: Option<String>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (label, means) = offer_words(offer, reach);
        let (job, call_id) = (id.clone(), call.to_string());
        labelled(
            div().id(SharedString::from(format!("offer:{call}:{label}"))).tooltip(tip(means)).child(
                button(look, SharedString::from(format!("offer-btn:{call}:{label}")), label, Tone::Secondary, false, hint, true)
                    .children(folder_name.map(|name| div().font_family(mono(look)).text_color(look.color(token::ACCENT)).child(SharedString::from(name))))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let (folder, kind) = extra.clone().map_or((None, None), |(folder, kind, only)| (Some(folder).filter(|_| only), kind));
                        this.permit(&job, &call_id, offer, folder, kind, window, cx)
                    })),
            ),
            Role::Group,
            label,
        )
        .into_any_element()
    }

    pub(super) fn permission(&self, id: &SharedString, ask: &ThreadAsk, look: &Look, cx: &Context<Self>) -> AnyElement {
        let call = &ask.call;
        let target = self.is_target(&format!("question:{}", call.call));
        let word = match call.reaches_to {
            Some(ReachesTo::Read) => "Permission to read",
            Some(ReachesTo::Write) => "Permission to write",
            Some(ReachesTo::Run) => "Permission to run",
            None => "Permission",
        };
        let allows_once = call.offers.is_empty() || call.offers.contains(&CallAnswer::AllowOnce);
        if let Some(page) = self.pages.get(id) {
            let (job, call_id) = (id.clone(), call.call.clone());
            let (job2, call_id2) = (id.clone(), call.call.clone());
            let refuses = call.offers.is_empty() || call.offers.contains(&CallAnswer::Refuse);
            page.keys.set(
                &format!("question:{}", call.call),
                Some(super::keys::Card {
                    at: usize::MAX / 2,
                    kind: word.to_string(),
                    title: first_line(&ask.command).to_string(),
                    yes: allows_once.then(|| -> (String, super::keys::Run) {
                        ("Allow once".into(), std::rc::Rc::new(move |this, window, cx| this.permit(&job, &call_id, CallAnswer::AllowOnce, None, None, window, cx)))
                    }),
                    change: None,
                    send_back: None,
                    no: refuses.then(|| -> (String, super::keys::Run) {
                        (if call.offers.is_empty() { "Deny".into() } else { offer_words(CallAnswer::Refuse, call.reaches.is_some()).0.to_string() }, std::rc::Rc::new(move |this, window, cx| this.permit(&job2, &call_id2, CallAnswer::Refuse, None, None, window, cx)))
                    }),
                }),
            );
        }
        let command_text = div().min_w_0().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(ask.command.clone()));
        let command = if allows_once {
            let (job, call_id) = (id.clone(), call.call.clone());
            labelled(
                div()
                    .id(SharedString::from(format!("run:{}", call.call)))
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .bg(look.color(token::BG_SUNKEN))
                    .child(command_text)
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .child(labelled(
                                div()
                                    .id(SharedString::from(format!("allow-once-icon:{}", call.call)))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .size(look.length(token::H_CONTROL_SM))
                                    .rounded(look.length(token::RADIUS_MD))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                                    .tooltip(tip("Allow once"))
                                    .child(glyph(look, "play", Size::Mark, look.color(token::FG_MUTED)))
                                    .on_click(cx.listener(move |this, _, window, cx| this.permit(&job, &call_id, CallAnswer::AllowOnce, None, None, window, cx))),
                                Role::Button,
                                "Allow once",
                            ))
                            .children(target.then(|| kbd(look, "\u{2325}\u{21b5}"))),
                    ),
                Role::Group,
                first_line(&ask.command).to_string(),
            )
            .into_any_element()
        } else {
            command_text.into_any_element()
        };
        let reach = call.reaches.is_some() && call.offers.iter().any(|offer| matches!(offer, CallAnswer::AlwaysAllow | CallAnswer::AlwaysBlock | CallAnswer::AllowForSession));
        let hint = |offer: CallAnswer| (target && offer == CallAnswer::Refuse).then_some("\u{2325}X");
        let answers: AnyElement = if call.offers.is_empty() {
            let (job, call_id) = (id.clone(), call.call.clone());
            let (job2, call_id2) = (id.clone(), call.call.clone());
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(labelled(
                    button(look, SharedString::from(format!("allow-once:{}", call.call)), "Allow once", Tone::Secondary, false, target.then_some("\u{2325}\u{21b5}"), true)
                        .on_click(cx.listener(move |this, _, window, cx| this.permit(&job, &call_id, CallAnswer::AllowOnce, None, None, window, cx))),
                    Role::Button,
                    "Allow once",
                ))
                .child(labelled(
                    button(look, SharedString::from(format!("deny:{}", call.call)), "Deny", Tone::Secondary, false, target.then_some("\u{2325}X"), true)
                        .on_click(cx.listener(move |this, _, window, cx| this.permit(&job2, &call_id2, CallAnswer::Refuse, None, None, window, cx))),
                    Role::Button,
                    "Deny",
                ))
                .into_any_element()
        } else if reach {
            self.reach_answers(id, call, target, look, cx)
        } else {
            let offers: Vec<AnyElement> = call.offers.iter().copied().filter(|offer| *offer != CallAnswer::AllowOnce).map(|offer| self.offer_button(id, &call.call, offer, false, hint(offer), None, None, look, cx)).collect();
            div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).children(offers).into_any_element()
        };
        let said = self.dock_of(id).and_then(|dock| dock.said.get(&format!("question:{}", call.call)).cloned());
        labelled(
            div()
                .id(SharedString::from(format!("permission:{}", call.call)))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(eyebrow(look, word).text_color(look.color(token::FG_MUTED)))
                .child(command)
                .children(said.map(|why| super::acts::refusal(look, &call.call, why)))
                .child(labelled(super::acts::gated(div().id(SharedString::from(format!("answers:{}", call.call))).child(answers), self.off_shown(cx)), Role::Group, "Answers")),
            Role::Region,
            "Waiting on you",
        )
        .into_any_element()
    }

    /// The reach card: the once answers, then what always applies to, and calls like this.
    fn reach_answers(&self, id: &SharedString, call: &CallInFlight, target: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let reaches = call.reaches.clone().unwrap_or_default();
        let parts: Vec<String> = reaches.split('/').filter(|part| !part.is_empty()).map(str::to_string).collect();
        let perm = self.perm_of(id, &call.call);
        let folder_at = perm.folder.unwrap_or(parts.len().saturating_sub(2)).min(parts.len().saturating_sub(1));
        let folder = format!("/{}", parts[..=folder_at].join("/"));
        let last_name = parts.get(folder_at).cloned().unwrap_or_default();
        let hue = look.color(token::ACCENT);
        let hint = |offer: CallAnswer| (target && offer == CallAnswer::Refuse).then_some("\u{2325}X");
        let once: Vec<AnyElement> = call
            .offers
            .iter()
            .copied()
            .filter(|offer| !matches!(offer, CallAnswer::AllowOnce | CallAnswer::AllowLikeThisForSession | CallAnswer::AlwaysAllow | CallAnswer::AlwaysBlock | CallAnswer::AllowForSession))
            .map(|offer| self.offer_button(id, &call.call, offer, true, hint(offer), None, None, look, cx))
            .collect();
        let always: Vec<AnyElement> = call
            .offers
            .iter()
            .copied()
            .filter(|offer| matches!(offer, CallAnswer::AlwaysAllow | CallAnswer::AlwaysBlock | CallAnswer::AllowForSession))
            .map(|offer| self.offer_button(id, &call.call, offer, true, None, Some((folder.clone(), None, true)), Some(last_name.clone()), look, cx))
            .collect();
        let mut picker: Vec<AnyElement> = Vec::new();
        for (at, part) in parts.iter().enumerate() {
            if at > 0 {
                picker.push(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("\u{203a}").into_any_element());
            }
            let key = call.call.clone();
            picker.push(
                labelled(
                    div()
                        .id(SharedString::from(format!("folder:{}:{at}", call.call)))
                        .flex()
                        .items_center()
                        .h(look.length(token::H_KBD))
                        .px(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(if at == folder_at { hue } else { look.color(token::BORDER_SUBTLE) })
                        .bg(if at == folder_at { look.color(token::ACCENT_MUTED) } else if at > folder_at { look.color(token::ACCENT_FAINT) } else { gpui::transparent_black() })
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .cursor_pointer()
                        .tooltip(tip(format!("/{}", parts[..=at].join("/"))))
                        .child(SharedString::from(part.clone()))
                        .on_click(cx.listener(move |this, _, _, cx| this.set_perm(&key, |perm| perm.folder = Some(at), cx))),
                    Role::Button,
                    part.clone(),
                )
                .into_any_element(),
            );
        }
        let applies = match call.reaches_to {
            Some(ReachesTo::Read) => Some(("reads", "of")),
            Some(ReachesTo::Write) => Some(("writes", "to")),
            Some(ReachesTo::Run) => Some(("commands", "on")),
            None => None,
        };
        let like = call.offers.contains(&CallAnswer::AllowLikeThisForSession).then(|| call.kind.clone()).flatten().map(|kind| self.like_this(id, call, kind, &perm, &folder, &last_name, look, cx));
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .child(labelled(super::acts::gated(div().id(SharedString::from(format!("answers-once:{}", call.call))).flex().flex_wrap().gap(look.length(token::SPACE_2)).children(once), self.off_shown(cx)), Role::Group, "Answers"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::ACCENT_MUTED))
                    .bg(look.color(token::ACCENT_FAINT))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_1))
                            .text_size(look.length(token::TEXT_XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child("Always applies to")
                            .children(applies.map(|(what, to)| div().flex().gap(look.length(token::SPACE_1)).child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(what)).child(to))),
                    )
                    .child(labelled(div().id(SharedString::from(format!("folders:{}", call.call))).flex().flex_wrap().items_center().gap(look.length(token::SPACE_1)).children(picker), Role::Group, "Folder for always"))
                    .child(div().font_family(mono(look)).text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)).child(format!("{folder}/*")))
                    .child(div().flex().flex_wrap().gap(look.length(token::SPACE_2)).children(always))
                    .children(like),
            )
            .into_any_element()
    }

    /// Calls like this one: the width of the kind, whether it holds only in the folder, and the answer.
    #[allow(clippy::too_many_arguments)]
    fn like_this(&self, id: &SharedString, call: &CallInFlight, kind: CallKind, perm: &Perm, folder: &str, last_name: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let hue = look.color(token::ACCENT);
        let word = |key: String, at: &str, said: &str, chosen: bool, wide: bool| {
            labelled(
                button(look, SharedString::from(format!("kind-{at}:{key}")), said, Tone::Ghost, false, None, true)
                    .when(chosen, |b| b.border_color(hue))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_perm(&key, |perm| perm.wide = wide, cx))),
                Role::Button,
                said.to_string(),
            )
        };
        let narrowed = CallKind { program: kind.program.clone(), script: if perm.wide { None } else { kind.script.clone() } };
        let (only, only_key) = (perm.only_in, call.call.clone());
        let folder = folder.to_string();
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .pt(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::ACCENT_MUTED))
            .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child("Calls like"))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("kind:{}", call.call)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(word(call.call.clone(), "program", &kind.program, perm.wide, true))
                    .children(kind.script.as_ref().map(|script| word(call.call.clone(), "script", script, !perm.wide, false))),
                Role::Group,
                "Calls like",
            ))
            .child(labelled(
                div()
                    .id(SharedString::from(format!("only-in:{}", call.call)))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .cursor_pointer()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(
                        div()
                            .w(look.length(token::SPACE_6))
                            .h(look.length(token::SPACE_4))
                            .rounded_full()
                            .bg(if only { hue } else { look.color(token::BG_SUNKEN) })
                            .border(look.length(token::BORDER_WIDTH))
                            .border_color(look.color(token::BORDER_DEFAULT))
                            .flex()
                            .items_center()
                            .when(only, |t| t.justify_end())
                            .child(div().size(look.length(token::SPACE_3)).rounded_full().bg(look.color(token::FG_DEFAULT))),
                    )
                    .child(format!("Only in {last_name}"))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_perm(&only_key, |perm| perm.only_in = !perm.only_in, cx))),
                Role::Switch,
                format!("Only in {last_name}"),
            ))
            .child(div().flex().child(self.offer_button(id, &call.call, CallAnswer::AllowLikeThisForSession, true, None, Some((folder, Some(narrowed), only)), None, look, cx)))
            .into_any_element()
    }
}

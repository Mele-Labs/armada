//! Report, in the toolbelt (`ReportBox.tsx`): a dialog over the whole window where the person types
//! what went wrong in the Job's own message box, with `@` files, `#` issues and files attached. Send
//! starts a Job in Armada's repository where Fleet serves one, else the reported Job's, and tells it
//! his words and what the app holds about this Job (`report_brief`), then opens it. Esc or Cancel
//! puts it away and keeps what was typed; a send clears it.
//!
//! The app has one message box, so while the dialog is open the box is drawn in it and writes under
//! the report's own key; the Job's box shows its kept draft under the scrim.

use gpui::{anchored, deferred, div, point, prelude::*, Anchor, AnyElement, Context, Focusable, FontWeight, MouseButton, Role, SharedString, Window};
use ipc::{SendSessionMessage, SendSessionMessageBody, SessionRecord, SessionTag, StartSession};

use super::composer::{encoded_files, Kept};
use super::head::act_button;
use super::offer::mention_of;
use super::report_brief::{report_brief, ReportFrom};
use super::rows::{glyph, Size};
use super::wire::Done;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// The Manifest a report's Job starts on where Fleet serves it: a fix to how a Job behaved is usually Armada's.
const ARMADA: &str = "armada";

/// The dialog, while it is open.
pub(super) struct Report {
    pub job: SharedString,
    /// The Job a report started, kept while its message is refused so a second press tells the same one.
    started: Option<String>,
    sending: bool,
    said: Option<SharedString>,
}

/// The repository's root, from where a Job runs: a slot's is under the repository's `.armada/slots`.
fn root_of(cwd: &str) -> String {
    cwd.find("/.armada/slots/").map_or(cwd, |at| &cwd[..at]).to_string()
}

impl JobView {
    /// The key the box writes under: the report's while it is open, else the shown Job's.
    pub(super) fn box_key(&self) -> Option<SharedString> {
        match &self.reporting {
            Some(report) => Some(format!("report:{}", report.job).into()),
            None => self.shown.clone(),
        }
    }

    /// The Manifest a report of `record` starts its Job on, where one is known.
    pub(super) fn report_manifest(&self, record: &SessionRecord, cx: &gpui::App) -> Option<ipc::ManifestId> {
        let records = self.list.read(cx).records();
        records.iter().filter_map(|job| job.manifest_id.as_ref()).find(|id| id.as_str().eq_ignore_ascii_case(ARMADA)).cloned().or_else(|| record.manifest_id.clone())
    }

    /// The toolbelt's Report: off, with its reason, where the reported Job's repository is not known.
    pub(super) fn report_act(&self, id: &SharedString, cx: &Context<Self>) -> AnyElement {
        let look = cx.look();
        let known = self.record(id, cx).is_some_and(|record| self.report_manifest(record, cx).is_some());
        let tip: SharedString = if known { "Report an issue with the Job".into() } else { "The repository is not known".into() };
        let open = self.reporting.is_some();
        labelled(
            act_button(look, "act-report", "bug", "report", tip, open, !known).when(known, |button| button.on_click(cx.listener(|this, _, window, cx| this.toggle_report(window, cx)))),
            Role::Button,
            "Report issue with job",
        )
        .into_any_element()
    }

    pub(super) fn toggle_report(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.reporting.is_some() {
            return self.close_report(window, cx);
        }
        let Some(job) = self.shown.clone() else { return };
        // The Job's words are kept under its own key, and the box takes the report's.
        self.composer_leave(cx);
        self.reporting = Some(Report { job, started: None, sending: false, said: None });
        let key = self.box_key().unwrap_or_default();
        let kept = self.draft_of(&key);
        self.set_box(&kept, window, cx);
        self.composer.input.update(cx, |input, cx| {
            let end = input.value().len();
            input.set_selected_range(end..end, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    /// Esc or Cancel: what was typed stays under the report's key, and the box takes the Job's words back.
    pub(super) fn close_report(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.composer_leave(cx);
        let shown = self.shown.clone();
        self.composer_enter(shown.as_ref(), window, cx);
        self.to_composer(window, cx);
        cx.notify();
    }

    /// Start the report's Job, once, then tell it the brief; a refusal keeps the dialog with Fleet's reason.
    pub(super) fn send_report(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(report) = self.reporting.as_ref() else { return };
        if report.sending {
            return;
        }
        let (job, started) = (report.job.clone(), report.started.clone());
        let key = self.box_key().unwrap_or_default();
        let kept = self.kept(cx);
        let files = self.composer.files_of(&key).to_vec();
        if kept.text.trim().is_empty() && files.is_empty() {
            return;
        }
        let Some(record) = self.record(&job, cx).cloned() else { return };
        let Some(manifest_id) = self.report_manifest(&record, cx) else { return };
        let items = self.thread(&job, cx).map(|held| held.items.clone()).unwrap_or_default();
        let from = ReportFrom {
            at: chrono::Utc::now(),
            screen: "Jobs, the Job's toolbelt".to_string(),
            root: root_of(&record.cwd),
            repository: record.manifest_id.as_ref().map(|id| id.as_str().to_string()),
            refused: self.pages.get(&job).and_then(|page| page.refused.as_ref()).map(|why| why.to_string()),
            lapsed: self.thread(&job, cx).and_then(|held| held.head.lapsed.clone()),
        };
        let brief = report_brief(&kept.text, &record, &items, &from);
        let mentions: Vec<SessionTag> = kept.tags.iter().filter_map(|tag| mention_of(&tag.id, &tag.label)).collect();
        if let Some(report) = self.reporting.as_mut() {
            report.sending = true;
            report.said = None;
        }
        cx.notify();
        match started {
            Some(id) => self.tell_report(id, brief, files, mentions, window, cx),
            None => {
                let start = StartSession {
                    manifest_id,
                    title: None,
                    model: None,
                    effort: None,
                    mode: None,
                    fork: None,
                    root: None,
                    request: None,
                    after: None,
                    held: false,
                    project_rows: Vec::new(),
                    on_your_words: None,
                };
                let done: Done = Box::new(move |this, answer, window, cx| match answer {
                    Ok(bytes) => match ipc::decode::<SessionRecord>("SessionRecord", &bytes) {
                        Ok(started) => {
                            let id = started.id.as_str().to_string();
                            if let Some(report) = this.reporting.as_mut() {
                                report.started = Some(id.clone());
                            }
                            this.tell_report(id, brief, files, mentions, window, cx);
                        }
                        // Mock data: Fleet answers nothing, and the report reads as sent.
                        Err(_) if this.fleet.is_none() => this.report_sent(None, window, cx),
                        Err(why) => this.report_refused(why.to_string().into(), cx),
                    },
                    Err(why) => this.report_refused(why.into(), cx),
                });
                self.post("/sessions/start".into(), ipc::encode(&start), window, cx, done);
            }
        }
    }

    fn tell_report(&mut self, id: String, brief: String, files: Vec<super::composer::Attached>, mentions: Vec<SessionTag>, window: &mut Window, cx: &mut Context<Self>) {
        let session_id = ipc::SessionId::carried(id.clone());
        cx.spawn_in(window, async move |this, cx| {
            let uploads = cx.background_executor().spawn(async move { encoded_files(files) }).await;
            let _ = this.update_in(cx, |this, window, cx| {
                let body = SendSessionMessageBody { message: SendSessionMessage { session_id, text: brief, attachments: uploads, mentions }, aim: None };
                let done: Done = Box::new(move |this, answer, window, cx| match answer {
                    Ok(_) => this.report_sent(Some(id), window, cx),
                    Err(why) => this.report_refused(why.into(), cx),
                });
                this.post("/sessions/message".into(), ipc::encode(&body), window, cx, done);
            });
        })
        .detach();
    }

    /// Sent: the report's words and files go, the dialog shuts, and the new Job opens.
    fn report_sent(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(key) = self.box_key().filter(|_| self.reporting.is_some()) {
            self.composer.box_of(&key).files.clear();
            self.set_box(&Kept::default(), window, cx);
            self.typed_now(cx);
        }
        self.close_report(window, cx);
        if let Some(id) = id {
            self.open_job(id, cx);
        }
    }

    fn report_refused(&mut self, why: SharedString, cx: &mut Context<Self>) {
        if let Some(report) = self.reporting.as_mut() {
            report.sending = false;
            report.said = Some(why);
        }
        cx.notify();
    }

    /// The dialog over the window: its glyph and title, the message box, and Cancel.
    pub(super) fn render_report(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let report = self.reporting.as_ref()?;
        let look = cx.look();
        let size = window.viewport_size();
        let key = self.box_key()?;
        let nothing = self.composer.input.read(cx).value().trim().is_empty() && self.composer.files_of(&key).is_empty();
        let focused = self.composer.input.read(cx).focus_handle(cx).contains_focused(window, cx);
        let send = if report.sending { div().opacity(0.4).child(self.send_button(true, window, cx)).into_any_element() } else { self.send_button(nothing, window, cx) };
        let bar = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_2))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(self.attach_button(look, cx))
            .child(div().flex_1())
            .child(self.render_talk(look, cx))
            .child(send);
        let frame = div()
            .relative()
            .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, _, cx| this.take_paths(paths.paths().to_vec(), cx)))
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(if focused { token::BORDER_DEFAULT } else { token::BORDER_SUBTLE }))
            .bg(look.color(token::BG_SUNKEN))
            .children(self.render_offer(look, cx))
            .children(self.render_tray(&key, look, cx))
            .child(labelled(self.box_words(look, cx), Role::TextInput, "Report"))
            .child(bar);
        let said = report.said.clone().map(|why| {
            labelled(
                div().id("report-refused").text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why),
                Role::Status,
                "Refused",
            )
        });
        let cancel = labelled(
            div()
                .id("report-cancel")
                .flex_none()
                .flex()
                .items_center()
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_4))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED))
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child("Cancel")
                .on_click(cx.listener(|this, _, window, cx| this.close_report(window, cx))),
            Role::Button,
            "Cancel",
        );
        let panel = div()
            .id("report")
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w_full()
            .max_w(look.length(token::W_DIALOG_WIDE))
            .max_h_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .p(look.length(token::PAD_CARD))
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .text_color(look.color(token::FG_DEFAULT))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(div().mt(look.length(token::SPACE_1)).child(glyph(look, "signpost", Size::Mark, look.color(token::ACCENT))))
                    .child(
                        div()
                            .min_w_0()
                            .text_size(look.length(token::TEXT_LG))
                            .line_height(look.length(token::LEADING_LG))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .child("Report an issue with the Job"),
                    ),
            )
            .child(div().pl(look.length(token::SPACE_4) + look.length(token::SPACE_2)).flex().flex_col().gap(look.length(token::SPACE_2)).child(frame).children(said))
            .child(div().flex_none().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel));
        Some(
            deferred(
                anchored().position(point(gpui::px(0.), gpui::px(0.))).anchor(Anchor::TopLeft).child(
                    labelled(
                        div()
                            .id("report-scrim")
                            .occlude()
                            .w(size.width)
                            .h(size.height)
                            .flex()
                            .items_center()
                            .justify_center()
                            .p(look.length(token::SPACE_6))
                            .bg(look.color(token::SCRIM))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_report(window, cx)))
                            .child(panel),
                        Role::Dialog,
                        "Report an issue with the Job",
                    ),
                ),
            )
            .with_priority(2)
            .into_any_element(),
        )
    }
}

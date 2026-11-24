//! The Docs card's Check, and the flag under a doc a press found disagreeing with the code. `crates/ipc/operations/check_docs.toml`.
//!
//! **Check runs only on his press**, and Fleet spends a call of the librarian's night share for each doc. A refusal stands in
//! the card's header line in Fleet's words. A flag answers with Start a Job, which opens the Job Fleet started, or Dismiss,
//! which keeps it quiet until the doc or the code changes.

use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, Role, SharedString, Task, Window};

use super::dossier::button;
use super::page::{FeaturePageView, Load, Region, Source};
use super::page_model::{Body, ReadRow, Section};
use super::wire;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::{refusal, tip};
use crate::settings::wire::Refused;
use crate::titlebar::parts::{glyph, mono, spinner};

/// Where the press of Check stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum Checking {
    #[default]
    Idle,
    /// The models are being asked.
    Working,
    /// Fleet refused it, or the share ran out before every doc, in a sentence for the header line.
    Said(String),
}

/// Where his answer to the flag the keys are on stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) enum FlagAnswer {
    #[default]
    Idle,
    /// The answer to the doc `target` is in flight.
    Working(String),
    /// Fleet refused the answer to the doc `target`, in its own words.
    Failed(String, String),
}

/// What the header line says where the share ran out before every doc was checked.
pub(super) const SHARE_RAN_OUT: &str = "The share ran out tonight before every doc was checked";

/// The flag's two lines and its question, as the screen and its labels say them.
pub(super) const DISAGREE: &str = "The doc and the code disagree";

impl FeaturePageView {
    /// The rows the Docs card holds.
    fn doc_rows(&self) -> &[ReadRow] {
        match self.cards.get(&Section::Docs) {
            Some(Body::Reads(rows)) => rows,
            _ => &[],
        }
    }

    /// Whether the page holds a doc that is a file of the repository, which is what a press of Check reads.
    pub(super) fn can_check(&self) -> bool {
        matches!(self.page, Load::Read(_)) && self.doc_rows().iter().any(|row| row.glyph == "file-text" && !row.gone)
    }

    /// The places in the Docs card of the docs that carry a flag.
    pub(super) fn flagged(&self) -> Vec<usize> {
        self.doc_rows().iter().enumerate().filter(|(_, row)| row.flag.is_some()).map(|(at, _)| at).collect()
    }

    /// The doc whose flag the keys are on, by its target.
    fn flag_target(&self) -> Option<String> {
        let flagged = self.flagged();
        let at = *flagged.get(self.flag_at.min(flagged.len().saturating_sub(1)))?;
        Some(self.doc_rows()[at].target.clone())
    }

    /// Arrows on a flag's buttons move to the flag of the doc before or after.
    pub(super) fn step_flag(&mut self, by: isize, cx: &mut Context<Self>) {
        let last = self.flagged().len().saturating_sub(1) as isize;
        let next = (self.flag_at as isize + by).clamp(0, last) as usize;
        if next != self.flag_at {
            crate::ui_sounds::moved();
            self.flag_at = next;
            cx.notify();
        }
    }

    /// Check: Fleet puts each doc to the model against the feature's code. The answer's checks go on the docs as they
    /// stand, so the page is not read again for them.
    pub(super) fn check_docs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.checking == Checking::Working || !self.can_check() {
            return;
        }
        self.engage();
        let body = ipc::CheckDocs { project: self.project.clone(), slug: self.slug.clone() };
        let ask = match &self.source {
            Source::Live(_) => match self.port(cx) {
                Some(port) => wire::check_docs(port, &body, cx),
                None => Task::ready(Err(Refused::said("Fleet is not connected"))),
            },
            Source::Mock => Task::ready(match self.mock_refusal.clone() {
                Some(said) => Err(Refused::said(said)),
                None => Ok(self.mock_checked(body)),
            }),
        };
        self.checking = Checking::Working;
        self.doc_work = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = ask.await;
            let _ = this.update_in(cx, |this, _, cx| this.checked(answer, cx));
        }));
        cx.notify();
    }

    /// The mock's answer: every doc that is a file, checked now, the way the mock page found them.
    fn mock_checked(&self, body: ipc::CheckDocs) -> ipc::DocsChecked {
        let now = ipc::Instant::carried(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        let checked = match &self.page {
            Load::Read(page) => page
                .docs
                .iter()
                .filter(|doc| !doc.target.contains("://"))
                .map(|doc| ipc::CheckedDoc { target: doc.target.clone(), check: ipc::DocCheck { at: now.clone(), disagrees: doc.check.as_ref().and_then(|check| check.disagrees.clone()), dismissed: false } })
                .collect(),
            _ => Vec::new(),
        };
        ipc::DocsChecked { project: body.project, slug: body.slug, checked, unchecked: Vec::new(), calls: 1 }
    }

    fn checked(&mut self, answer: Result<ipc::DocsChecked, Refused>, cx: &mut Context<Self>) {
        match answer {
            Ok(checked) => {
                self.checking = if checked.unchecked.is_empty() { Checking::Idle } else { Checking::Said(SHARE_RAN_OUT.to_string()) };
                if let Load::Read(page) = &mut self.page {
                    for one in checked.checked {
                        if let Some(doc) = page.docs.iter_mut().find(|doc| doc.target == one.target) {
                            doc.check = Some(one.check);
                        }
                    }
                }
                self.flag_at = 0;
                self.rebuild(cx);
            }
            Err(why) => {
                self.checking = Checking::Said(why.said);
                cx.notify();
            }
        }
    }

    /// The body of an answer to the flag the keys are on, where one is drawn and no answer is in flight.
    fn flag_body(&self) -> Option<ipc::DocFlag> {
        if matches!(self.flag_answer, FlagAnswer::Working(_)) {
            return None;
        }
        Some(ipc::DocFlag { project: self.project.clone(), slug: self.slug.clone(), target: self.flag_target()? })
    }

    /// Dismiss: the flag goes quiet until the doc or the code changes.
    pub(super) fn dismiss_doc_flag(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(body) = self.flag_body() else { return };
        self.engage();
        let ask = match &self.source {
            Source::Live(_) => match self.port(cx) {
                Some(port) => wire::dismiss_doc_flag(port, &body, cx),
                None => Task::ready(Err(Refused::said("Fleet is not connected"))),
            },
            Source::Mock => Task::ready(match self.mock_refusal.clone() {
                Some(said) => Err(Refused::said(said)),
                None => Ok(body.clone()),
            }),
        };
        self.flag_answer = FlagAnswer::Working(body.target.clone());
        self.doc_work = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = ask.await;
            let _ = this.update_in(cx, |this, window, cx| this.dismissed(body.target, answer, window, cx));
        }));
        cx.notify();
    }

    fn dismissed(&mut self, target: String, answer: Result<ipc::DocFlag, Refused>, window: &mut Window, cx: &mut Context<Self>) {
        match answer {
            Ok(_) => {
                self.flag_answer = FlagAnswer::Idle;
                if let Load::Read(page) = &mut self.page {
                    if let Some(check) = page.docs.iter_mut().find(|doc| doc.target == target).and_then(|doc| doc.check.as_mut()) {
                        check.dismissed = true;
                    }
                }
                let on_flag = matches!(self.region_in(window), Region::FlagStart | Region::FlagDismiss);
                self.flag_at = 0;
                self.rebuild(cx);
                // The buttons went with the flag: the keys go on the next flag, else back to Check.
                if on_flag {
                    let next = if self.flagged().is_empty() { Region::Check } else { Region::FlagStart };
                    self.focus_region(next, window, cx);
                }
            }
            Err(why) => {
                self.flag_answer = FlagAnswer::Failed(target, why.said);
                cx.notify();
            }
        }
    }

    /// Start a Job: Fleet starts one that quotes the doc and the code and asks him which is right, and the Job opens.
    pub(super) fn start_doc_job(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(body) = self.flag_body() else { return };
        self.engage();
        let ask = match &self.source {
            Source::Live(_) => match self.port(cx) {
                Some(port) => wire::start_doc_job(port, &body, cx),
                None => Task::ready(Err(Refused::said("Fleet is not connected"))),
            },
            Source::Mock => Task::ready(match self.mock_refusal.clone() {
                Some(said) => Err(Refused::said(said)),
                None => Ok(ipc::DocJobStarted {
                    project: body.project.clone(),
                    slug: body.slug.clone(),
                    target: body.target.clone(),
                    session_id: ipc::SessionId::carried("job-doc-or-code"),
                    title: "Doc or code".into(),
                    brief: String::new(),
                    send_failed: None,
                }),
            }),
        };
        self.flag_answer = FlagAnswer::Working(body.target.clone());
        self.doc_work = Some(cx.spawn_in(window, async move |this, cx| {
            let answer = ask.await;
            let _ = this.update_in(cx, |this, _, cx| this.job_started(body.target, answer, cx));
        }));
        cx.notify();
    }

    fn job_started(&mut self, target: String, answer: Result<ipc::DocJobStarted, Refused>, cx: &mut Context<Self>) {
        match answer {
            Ok(started) => {
                self.flag_answer = FlagAnswer::Idle;
                cx.notify();
                let job = started.session_id.as_str().to_string();
                if let Some(sink) = cx.try_global::<crate::titlebar::AskedSink>().cloned() {
                    (sink.0)(&crate::titlebar::Asked::OpenJob(job), cx);
                    crate::palette::show_jobs(cx);
                }
            }
            Err(why) => {
                self.flag_answer = FlagAnswer::Failed(target, why.said);
                cx.notify();
            }
        }
    }

    // ---- drawing -----------------------------------------------------------------------------

    /// The Docs card's title row: the title, the header line where Fleet refused or the share ran out, and Check, which reads
    /// Checking beside a spinner while the models are asked.
    pub(super) fn docs_head(&self, look: &Look, cx: &mut Context<Self>) -> AnyElement {
        let title = Section::Docs.title();
        let working = self.checking == Checking::Working;
        let said = match &self.checking {
            Checking::Said(said) => Some(labelled(div().id("feature-docs-said").flex_1().min_w_0().child(refusal(look, said.clone()).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS))), Role::Status, said.clone())),
            _ => None,
        };
        let check = self.can_check().then(|| {
            let words = if working { "Checking" } else { "Check" };
            let press = button(look, "feature-docs-check", words, false, false, working, Some(&self.check))
                .tooltip(tip("Ask the model whether each doc agrees with the code; one call of tonight's share a doc"))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.select(Section::Docs, cx);
                    this.focus_region(Region::Check, window, cx);
                    this.check_docs(window, cx);
                }));
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1) + px(2.))
                .children(working.then(|| spinner("feature-docs-check-spin", look.length(token::SPACE_3), look.color(token::STATUS_RUNNING), cx)))
                .child(labelled(press, Role::Button, words))
        });
        let head = div()
            .id("feature-card-title-docs")
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_3) + px(2.))
            .py(look.length(token::SPACE_2) - px(1.))
            .min_h(look.length(token::LEADING_XS) + (look.length(token::SPACE_2) + px(2.)) * 2.)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .rounded_t(crate::reviews::card_inner_radius(look))
            .bg(super::dossier::tint(look, look.color(token::BG_SUNKEN), token::BG_BASE, 0.60))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(labelled(div().id("feature-docs-title").flex_none().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title), Role::Heading, title))
            .children(said)
            .when(self.checking == Checking::Idle || working, |head| head.child(div().flex_1()))
            .children(check);
        head.into_any_element()
    }

    /// The flag under the doc at `at` of the Docs card: the awaiting review wash with a hairline all round, the glyph and the
    /// line that says they disagree, the doc's words, the code's place and words, then Start a Job and Dismiss.
    pub(super) fn render_doc_flag(&self, at: usize, row: &ReadRow, look: &Look, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let found = row.flag.as_ref()?;
        let flagged = self.flagged();
        let keyed = flagged.get(self.flag_at.min(flagged.len().saturating_sub(1))) == Some(&at);
        let working = matches!(&self.flag_answer, FlagAnswer::Working(target) if *target == row.target);
        let hue = token::STATUS_AWAITING_REVIEW;
        let small = |id: String, words: &'static str, label: &'static str| {
            div().id(SharedString::from(id)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).flex_none().child(words).tooltip(tip(label))
        };
        // The label stands to the left and what it labels wraps beside it, so a long quote hangs under its own first word.
        let quote = |text: String| div().min_w_0().text_color(look.color(token::FG_MUTED)).child(SharedString::from(text));
        let line = || div().flex().items_start().gap(look.length(token::SPACE_1) + px(2.)).text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_XS));
        let doc = line().child(small(format!("feature-flag-doc-{at}"), "Doc:", "What the doc says")).child(div().flex_1().min_w_0().child(quote(format!("\"{}\"", found.doc_quote))));
        let code = line().child(small(format!("feature-flag-code-{at}"), "Code:", "Where the code says otherwise, and what it does there")).child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().font_family(mono(look)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(found.code_at.clone())))
                .child(quote(found.code_quote.clone())),
        );
        let start = labelled(
            button(look, SharedString::from(format!("feature-flag-start-{at}")), "Start a Job", true, false, working, keyed.then_some(&self.flag_start))
                .tooltip(tip("Start a Job that quotes both and asks you which is right before it changes anything"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.select(Section::Docs, cx);
                    this.flag_at = this.flagged().iter().position(|one| *one == at).unwrap_or(0);
                    this.focus_region(Region::FlagStart, window, cx);
                    this.start_doc_job(window, cx);
                })),
            Role::Button,
            "Start a Job",
        );
        let dismiss = labelled(
            button(look, SharedString::from(format!("feature-flag-dismiss-{at}")), "Dismiss", false, false, working, keyed.then_some(&self.flag_dismiss))
                .tooltip(tip("Quiet this until the doc or the code changes"))
                .on_click(cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    this.select(Section::Docs, cx);
                    this.flag_at = this.flagged().iter().position(|one| *one == at).unwrap_or(0);
                    this.focus_region(Region::FlagDismiss, window, cx);
                    this.dismiss_doc_flag(window, cx);
                })),
            Role::Button,
            "Dismiss",
        );
        let refused = match &self.flag_answer {
            FlagAnswer::Failed(target, said) if *target == row.target => Some(labelled(div().id(SharedString::from(format!("feature-flag-refused-{at}"))).child(refusal(look, said.clone())), Role::Label, said.clone())),
            _ => None,
        };
        let ring = keyed && (self.flag_start.is_focused(window) || self.flag_dismiss.is_focused(window));
        let mark = labelled(div().id(SharedString::from(format!("feature-flag-mark-{at}"))).flex_none().tooltip(tip(DISAGREE)).child(glyph("triangle-alert", look.length(token::SPACE_3), look.color(hue))), Role::Image, DISAGREE);
        let flag = div()
            .id(SharedString::from(format!("feature-flag-{at}")))
            .mt(look.length(token::SPACE_1) + px(2.))
            .mb(px(2.))
            .px(look.length(token::SPACE_2) + px(2.))
            .py(look.length(token::SPACE_2))
            .flex()
            .flex_col()
            .gap(px(2.))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if ring { crate::settings::view::ring_colour(look) } else { look.color(hue) })
            .bg(look.color(token::STATUS_AWAITING_REVIEW_BG))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1) + px(2.))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(mark)
                    .child(DISAGREE),
            )
            .child(doc)
            .child(code)
            .children(refused)
            .child(div().mt(look.length(token::SPACE_1) + px(2.)).flex().items_center().gap(look.length(token::SPACE_2)).child(start).child(dismiss));
        Some(labelled(flag, Role::Region, format!("{DISAGREE}: {}", row.title)).into_any_element())
    }
}

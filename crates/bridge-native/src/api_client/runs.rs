//! The runs under a saved request: each send of it, by the person or a Job's agent, newest first, read a page at a time.
//! `GET /requests/runs` lists them and `GET /requests/runs/one` opens one, with the answer's headers. The runs belong to the
//! saved request that is open, so another request open puts them away and a run opened keeps them.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{RequestRun, RequestRunSummary};

use super::model::{self, RunsShown};
use super::view::{glyph, row_shell};
use super::{wire, ApiClient, Open};
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::settings::view::{refusal, tip};
use crate::titlebar::parts::mono;

/// Where reading a request's runs has got to.
pub(crate) enum RunsState {
    /// The first page, with none drawn yet.
    Loading,
    /// A page after the runs drawn.
    More,
    Ready,
    /// Fleet's own words for refusing the list, or the one run opened.
    Failed(String),
}

/// A saved request's runs as far as they are read.
pub(crate) struct Runs {
    pub collection: String,
    pub request: String,
    pub runs: Vec<RequestRunSummary>,
    /// Where the next page begins, as Fleet gave it; none on the last page.
    pub next: Option<String>,
    pub state: RunsState,
    task: Option<gpui::Task<()>>,
}

impl Runs {
    pub fn shown(&self) -> RunsShown<'_> {
        RunsShown {
            collection: &self.collection,
            request: &self.request,
            count: self.runs.len(),
            loading: matches!(self.state, RunsState::Loading | RunsState::More),
            failed: match &self.state {
                RunsState::Failed(said) => Some(said.as_str()),
                _ => None,
            },
        }
    }
}

impl ApiClient {
    /// The open request's runs: a saved request has them read from the top, a run opened keeps what is drawn, and any other
    /// request puts them away.
    pub(crate) fn expand_runs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (collection, request) = match &self.open {
            Open::Saved(collection, request) => (collection.clone(), request.clone()),
            Open::Run(_) => return,
            _ => {
                self.runs = None;
                return;
            }
        };
        let same = self.runs.as_ref().is_some_and(|runs| runs.collection == collection && runs.request == request);
        if !same {
            self.runs = Some(Runs { collection, request, runs: Vec::new(), next: None, state: RunsState::Loading, task: None });
        }
        self.fetch_runs(None, window, cx);
    }

    /// A page of the runs: the first replaces what is drawn, a later one follows it.
    fn fetch_runs(&mut self, before: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(runs) = self.runs.as_mut() else { return };
        let (collection, request) = (runs.collection.clone(), runs.request.clone());
        if before.is_some() {
            runs.state = RunsState::More;
        }
        let job = self.job.to_string();
        let (to_ask, from) = ((collection.clone(), request.clone(), before.clone()), before);
        let task = self.ask(
            to_ask,
            move |port, cx, (collection, request, before)| wire::runs(port, &job, &collection, &request, before.as_deref(), cx),
            |store, (collection, request, before)| Ok(store.runs(&collection, &request, before.as_deref())),
            move |this, answer, _, cx| {
                // The request moved on while the page was coming: it is not this one's.
                let Some(runs) = this.runs.as_mut().filter(|runs| runs.collection == collection && runs.request == request) else { return };
                runs.task = None;
                match answer {
                    Ok(page) => {
                        if from.is_some() {
                            let held: Vec<String> = runs.runs.iter().map(|run| run.id.clone()).collect();
                            runs.runs.extend(page.runs.into_iter().filter(|run| !held.contains(&run.id)));
                        } else {
                            runs.runs = page.runs;
                        }
                        runs.next = page.next;
                        runs.state = RunsState::Ready;
                    }
                    Err(said) => runs.state = RunsState::Failed(said),
                }
                this.relist(false);
                cx.notify();
            },
            window,
            cx,
        );
        if let Some(runs) = self.runs.as_mut() {
            // A page that answered at once has cleared its own task; a held one is kept so it is not cancelled.
            if matches!(runs.state, RunsState::Loading | RunsState::More) {
                runs.task = task;
            }
        }
        self.relist(false);
        cx.notify();
    }

    /// The last row of the list came into view: the next page of the runs, where there is one.
    pub(crate) fn want_more_runs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(runs) = self.runs.as_ref() else { return };
        if !matches!(runs.state, RunsState::Ready) || runs.task.is_some() {
            return;
        }
        if let Some(next) = runs.next.clone() {
            self.fetch_runs(Some(next), window, cx);
        }
    }

    /// A run pressed: its request and its answer, with the answer's headers, in the editor.
    pub(crate) fn open_run(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.runs.as_ref().and_then(|runs| runs.runs.get(at)).map(|run| run.id.clone()) else { return };
        let job = self.job.to_string();
        self.opening = self.ask(
            id,
            move |port, cx, id| wire::run(port, &job, &id, cx),
            |store, id| store.run(&id),
            |this, answer, window, cx| {
                this.opening = None;
                match answer {
                    Ok(run) => this.run_landed(run, window, cx),
                    Err(said) => {
                        if let Some(runs) = this.runs.as_mut() {
                            runs.state = RunsState::Failed(said);
                        }
                        this.relist(false);
                        cx.notify();
                    }
                }
            },
            window,
            cx,
        );
    }

    fn run_landed(&mut self, run: RequestRun, window: &mut Window, cx: &mut Context<Self>) {
        let start = model::run_start(&run).rebased(self.environments());
        let sent = model::run_sent(&run);
        if let Some(runs) = self.runs.as_mut() {
            if matches!(runs.state, RunsState::Failed(_)) {
                runs.state = RunsState::Ready;
            }
        }
        self.fill(Open::Run(run.id.clone()), start, Some(sent), window, cx);
        self.focus_address(window, cx);
    }

    // ---- drawing ---------------------------------------------------------------------------------

    /// One row under a saved request: how it ended, its code, how long, who sent it and when. Each is a fixed column.
    pub(crate) fn run_line(&self, look: &Look, at: usize, line: usize, held: bool, under_keys: bool, cx: &mut Context<Self>) -> AnyElement {
        let Some(run) = self.runs.as_ref().and_then(|runs| runs.runs.get(at)) else { return div().into_any_element() };
        let (icon, ink) = model::outcome_of(run.status).look();
        let said = model::status_said(run.status, run.error.as_deref());
        let who = model::run_who(run, &self.job);
        let when = crate::settings::time::stamp_at(run.at.as_str()).unwrap_or_default();
        let code = run.status.map(|status| status.to_string()).unwrap_or_default();
        let name = format!("{who}: {} {when}", if code.is_empty() { "No answer".to_string() } else { code.clone() });
        let mark = div().id(SharedString::from(format!("api-run-mark-{at}"))).flex_none().child(glyph(look, icon, ink)).tooltip(tip(said));
        let row = row_shell(look, format!("api-run-{at}"), held, under_keys)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.cursor = Some(line);
                this.open_run(at, window, cx);
            }))
            .child(div().flex_none().w(look.length(token::SPACE_4)))
            .child(mark)
            .child(div().flex_none().w(look.length(token::SPACE_8)).font_family(mono(look)).text_color(look.color(ink)).child(SharedString::from(code)))
            .child(div().flex_none().w(look.length(token::SPACE_12)).font_family(mono(look)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(model::time_of(run.ms))))
            .child(div().flex_1().min_w_0().truncate().child(SharedString::from(who)))
            .child(div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(when)));
        div().w_full().px(look.length(token::SPACE_2)).child(labelled(row, Role::ListBoxOption, name)).into_any_element()
    }

    /// The row that stands where the runs are still coming: bones in the columns.
    pub(crate) fn runs_loading_line(&self, look: &Look) -> AnyElement {
        use crate::settings::view::bone;
        labelled(
            div()
                .id("api-runs-loading")
                .w_full()
                .h(look.length(token::H_ROW))
                .px(look.length(token::SPACE_4) + look.length(token::SPACE_2))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(div().flex_none().w(look.length(token::SPACE_4)))
                .child(bone(look, look.length(token::SPACE_3), look.length(token::SPACE_3)))
                .child(bone(look, look.length(token::SPACE_8), look.length(token::LEADING_XS)))
                .child(bone(look, look.length(token::SPACE_12), look.length(token::LEADING_XS))),
            Role::Image,
            "Loading runs",
        )
        .into_any_element()
    }

    /// Fleet's words for refusing the list or the run, under the saved request.
    pub(crate) fn runs_failed_line(&self, look: &Look, said: String) -> AnyElement {
        div()
            .id("api-runs-failed")
            .w_full()
            .px(look.length(token::SPACE_4) + look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
            .child(refusal(look, said))
            .into_any_element()
    }
}

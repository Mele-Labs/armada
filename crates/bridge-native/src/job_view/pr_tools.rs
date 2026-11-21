//! The pull request panel's Review tab runs review tools by hand: pick one, Start, see it run, read its
//! findings in place. Fleet lists the tools (`GET /review_tools`), starts the run (`POST /review_runs`)
//! and says each change of state as `review_run.changed`; the Reviews screen's tool list, run landing and
//! markdown are used as they stand (`reviews::shared`). A run is read-only and starts only from Start.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Toggled, Window};
use ipc::{ReviewFinding, ReviewResult, ReviewRun, ReviewRunKind, ReviewRunState, ReviewRuns, ReviewSeverity, ReviewTools, StartReviewRun};

use super::pr_panel::{small_button, PullPanel, PullTab};
use super::pulls::state_chip;
use super::rows::{glyph, mono, tip, Size};
use super::wire::encoded;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::reviews::shared;

/// A review tool he can pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Pick {
    pub id: String,
    pub name: String,
}

/// The tab's tools and the runs on this pull request.
#[derive(Default)]
pub(super) struct Tools {
    /// The repository's tools as Fleet listed them, or why it did not.
    pub picks: Option<Result<Vec<Pick>, String>>,
    /// The tool ticked. None is the first.
    pub pick: Option<String>,
    /// Every run kept or going for this pull request.
    pub runs: Option<Vec<ReviewRun>>,
    asked_picks: bool,
    asked_runs: bool,
    /// Start was pressed and Fleet has not answered.
    pub starting: bool,
    /// Fleet's reason for a refused start.
    pub said: Option<String>,
}

impl Tools {
    /// The tool Start runs: the one ticked, else the first.
    pub(super) fn picked(&self) -> Option<&Pick> {
        let picks = self.picks.as_ref()?.as_ref().ok()?;
        self.pick.as_ref().and_then(|id| picks.iter().find(|one| one.id == *id)).or_else(|| picks.first())
    }
}

/// A run's state as a chip: the registry glyph, its word, its hue and what the word is short for.
fn state_look(state: ReviewRunState) -> (&'static str, &'static str, Token, &'static str) {
    match state {
        ReviewRunState::Queued => ("clock", "Queued", token::FG_MUTED, "Waiting for a place to read the pull request in"),
        ReviewRunState::Running => ("loader-circle", "Running", token::STATUS_RUNNING, "Reading the pull request"),
        ReviewRunState::Done => ("circle-check", "Done", token::RUN_PASSED, "Read the pull request and answered"),
        ReviewRunState::Failed => ("circle-x", "Failed", token::RUN_FAILED, "Ended without an answer"),
    }
}

/// A finding's severity as a chip: red for high, amber for medium, blue for low, as the Reviews screen hues its flags.
fn severity_look(severity: ReviewSeverity) -> (&'static str, &'static str, Token, &'static str) {
    match severity {
        ReviewSeverity::High => ("circle-x", "High", token::RUN_FAILED, "A high severity finding"),
        ReviewSeverity::Medium => ("triangle-alert", "Medium", token::STATUS_AWAITING_REVIEW, "A medium severity finding"),
        ReviewSeverity::Low => ("info", "Low", token::ACCENT, "A low severity finding"),
    }
}

/// The latest run of each tool at `head`, newest first. Every head where the page has none yet.
pub(super) fn tool_runs<'a>(runs: &'a [ReviewRun], head: Option<&str>) -> Vec<&'a ReviewRun> {
    let mut latest: Vec<&ReviewRun> = Vec::new();
    for run in runs.iter().filter(|run| run.kind == ReviewRunKind::Tool && run.since.is_none() && head.is_none_or(|head| run.head == head)) {
        match latest.iter().position(|held| held.name == run.name) {
            Some(at) if latest[at].started_at < run.started_at => latest[at] = run,
            Some(_) => {}
            None => latest.push(run),
        }
    }
    latest.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    latest
}

impl JobView {
    /// The repository's tools and this pull request's runs, asked once the Review tab is in front.
    pub(super) fn want_pull_tools(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel().filter(|panel| panel.tab == PullTab::Review) else { return };
        let (Some(manifest), number) = (panel.pull.manifest.clone(), panel.pull.number) else { return };
        let (want_picks, want_runs) = (!panel.tools.asked_picks, !panel.tools.asked_runs);
        if self.fleet.is_none() {
            if let Some(panel) = self.pull_panel_mut() {
                panel.tools.asked_picks = true;
                panel.tools.asked_runs = true;
                if want_picks {
                    panel.tools.picks = Some(Ok(shared::mock_tools().into_iter().map(|tool| Pick { id: tool.id, name: tool.name }).collect()));
                }
                if want_runs {
                    panel.tools.runs.get_or_insert_with(Vec::new);
                }
            }
            return;
        }
        if let Some(panel) = self.pull_panel_mut() {
            panel.tools.asked_picks = true;
            panel.tools.asked_runs = true;
        }
        if want_picks {
            self.get(
                format!("/review_tools/{}", encoded(&manifest)),
                window,
                cx,
                Box::new(move |this, answer, _, cx| {
                    let read = answer.and_then(|bytes| ipc::decode::<ReviewTools>("ReviewTools", &bytes).map_err(|why| why.to_string())).map(|tools| {
                        shared::tools_of(&tools).into_iter().map(|tool| Pick { id: tool.id, name: tool.name }).collect::<Vec<_>>()
                    });
                    if let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) {
                        panel.tools.picks = Some(read);
                    }
                    cx.notify();
                }),
            );
        }
        if want_runs {
            self.get(
                format!("/review_runs/{}/{number}", encoded(&manifest)),
                window,
                cx,
                Box::new(move |this, answer, _, cx| {
                    let read = answer.and_then(|bytes| ipc::decode::<ReviewRuns>("ReviewRuns", &bytes).map_err(|why| why.to_string()));
                    if let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) {
                        // Runs that could not be read stand as none: the next one that moves lands on its own.
                        panel.tools.runs = Some(read.map(|runs| runs.runs).unwrap_or_default());
                    }
                    cx.notify();
                }),
            );
        }
    }

    pub(super) fn pick_pull_tool(&mut self, id: String, cx: &mut Context<Self>) {
        if let Some(panel) = self.pull_panel_mut() {
            panel.tools.pick = Some(id);
        }
        cx.notify();
    }

    /// Start: the ticked tool runs on this pull request, again whatever is kept. Fleet answers the run at
    /// once, and the rest arrives as `review_run.changed`.
    pub(super) fn start_pull_tool(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel().filter(|panel| !panel.tools.starting) else { return };
        let (Some(manifest), number) = (panel.pull.manifest.clone(), panel.pull.number) else { return };
        let Some(tool) = panel.tools.picked().cloned() else { return };
        let head = panel.page().and_then(|page| page.head.clone());
        if let Some(panel) = self.pull_panel_mut() {
            panel.tools.starting = true;
            panel.tools.said = None;
        }
        if self.fleet.is_none() {
            let run = mock_run(&manifest, number, head.as_deref().unwrap_or("mock"), &tool);
            self.land_pull_run(run);
            if let Some(panel) = self.pull_panel_mut() {
                panel.tools.starting = false;
            }
            cx.notify();
            return;
        }
        let body = ipc::encode(&StartReviewRun { number, kind: ReviewRunKind::Tool, name: tool.id.clone(), again: true, place: None, finding: None, since: None });
        self.post(
            format!("/review_runs/{}", encoded(&manifest)),
            body,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let read = answer.and_then(|bytes| ipc::decode::<ReviewRun>("ReviewRun", &bytes).map_err(|why| why.to_string()));
                match read {
                    Ok(run) => {
                        this.land_pull_run(run);
                    }
                    Err(why) => {
                        if let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) {
                            panel.tools.said = Some(why);
                        }
                    }
                }
                if let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) {
                    panel.tools.starting = false;
                }
                cx.notify();
            }),
        );
        cx.notify();
    }

    /// A run lands on the open panel when it is this pull request's.
    pub(super) fn land_pull_run(&mut self, run: ReviewRun) -> bool {
        let Some(panel) = self.pull_panel_mut() else { return false };
        if panel.pull.manifest.as_deref() != Some(run.manifest_id.as_str()) || panel.pull.number != run.number {
            return false;
        }
        shared::land_run(panel.tools.runs.get_or_insert_with(Vec::new), run);
        true
    }

    /// `review_run.changed`: each run lands on the open pull request, and the panel notifies once for the batch. A gap
    /// in the stream reads its runs again.
    pub(super) fn review_runs_moved(&mut self, moved: &crate::fleet::ReviewRunsMoved, window: &mut Window, cx: &mut Context<Self>) {
        let mut landed = false;
        for run in &moved.runs {
            landed |= self.land_pull_run(run.clone());
        }
        if moved.lost {
            if let Some(panel) = self.pull_panel_mut() {
                panel.tools.asked_runs = false;
            }
            self.want_pull_tools(window, cx);
        }
        if landed || moved.lost {
            cx.notify();
        }
    }

    /// Review tools: the tools to pick from and Start on one line, then a card for each tool's run.
    pub(super) fn pull_tools(&self, panel: &PullPanel, look: &Look, cx: &Context<Self>) -> AnyElement {
        let muted = look.color(token::FG_MUTED);
        let tools = &panel.tools;
        let picked = tools.picked().map(|one| one.id.clone());
        let line = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        let line = match &tools.picks {
            None => line.child(super::pr_panel::stand_ins(look, "pull-tools-reading", &[0.4])),
            Some(Err(why)) => line.child(div().text_size(look.length(token::TEXT_XS)).text_color(muted).child(SharedString::from(why.clone()))),
            Some(Ok(picks)) if picks.is_empty() => line.child(div().text_size(look.length(token::TEXT_XS)).text_color(muted).child("No review tool is set up for this repository")),
            Some(Ok(picks)) => {
                let chips: Vec<AnyElement> = picks
                    .iter()
                    .map(|tool| {
                        let on = picked.as_deref() == Some(tool.id.as_str());
                        let (id, hue) = (tool.id.clone(), look.color(token::ACCENT));
                        let (ground, ink) = if on { (hue.opacity(0.16), hue) } else { (look.color(token::BG_HOVER), look.color(token::FG_MUTED)) };
                        labelled(
                            div()
                                .id(SharedString::from(format!("pull-tool:{}", tool.id)))
                                .flex_none()
                                .flex()
                                .items_center()
                                .h(look.length(token::H_CONTROL_SM))
                                .px(look.length(token::SPACE_3))
                                .rounded(look.length(token::RADIUS_MD))
                                .bg(ground)
                                .text_color(ink)
                                .text_size(look.length(token::TEXT_XS))
                                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                                .cursor_pointer()
                                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                                .on_click(cx.listener(move |this, _, _, cx| this.pick_pull_tool(id.clone(), cx)))
                                .child(SharedString::from(tool.name.clone())),
                            Role::RadioButton,
                            tool.name.clone(),
                        )
                        .into_any_element()
                    })
                    .collect();
                let can = picked.is_some() && !tools.starting;
                let start = labelled(
                    small_button(look, "pull-tool-start", "Start", true, can)
                        .tooltip(tip("Reads this pull request with the tool, and keeps what it finds. Nothing is posted"))
                        .on_click(cx.listener(|this, _, window, cx| this.start_pull_tool(window, cx))),
                    Role::Button,
                    "Start",
                );
                line.child(labelled(div().id("pull-tool-picks").flex().flex_wrap().gap(look.length(token::SPACE_2)).children(chips), Role::RadioGroup, "Review tool")).child(start)
            }
        };
        let head = panel.page().and_then(|page| page.head.as_deref());
        let runs: Vec<AnyElement> = tools.runs.as_deref().map(|runs| tool_runs(runs, head)).unwrap_or_default().into_iter().map(|run| self.pull_run(run, look, cx)).collect();
        let refused = tools.said.clone().map(|why| {
            labelled(
                div().id("pull-tool-refused").flex().items_center().gap(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).text_color(muted)
                    .child(glyph(look, "circle-x", Size::Small, look.color(token::RUN_FAILED)))
                    .child(SharedString::from(why.clone())),
                Role::Status,
                why,
            )
        });
        labelled(
            div().id("pull-tools").flex().flex_col().gap(look.length(token::SPACE_3)).child(line).children(refused).children(runs),
            Role::Region,
            "Review tools",
        )
        .into_any_element()
    }

    /// One tool's run: its state as a chip, its name and Run again, then what it found or why it failed.
    fn pull_run(&self, run: &ReviewRun, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (icon, word, hue, said) = state_look(run.state);
        let key = run.name.clone();
        let chip = {
            let tooltip = run.failed.clone().filter(|_| run.state == ReviewRunState::Failed).unwrap_or_else(|| said.to_string());
            labelled(state_chip(look, format!("pull-run-state:{key}"), icon, tooltip, word, hue), Role::Status, format!("{} {}", run.name, word.to_lowercase()))
        };
        let again = matches!(run.state, ReviewRunState::Done | ReviewRunState::Failed).then(|| {
            let id = run.name.clone();
            labelled(
                small_button(look, format!("pull-run-again:{key}"), "Run again", false, true).on_click(cx.listener(move |this, _, window, cx| {
                    this.pick_pull_tool(id.clone(), cx);
                    this.start_pull_tool(window, cx);
                })),
                Role::Button,
                format!("Run {} again", run.name),
            )
        });
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(chip)
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(run.name.clone())))
            .children(again);
        let body: Option<AnyElement> = match (run.state, &run.result) {
            (ReviewRunState::Failed, _) => {
                run.failed.clone().map(|why| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(why)).into_any_element())
            }
            (ReviewRunState::Done, Some(ReviewResult::Findings { findings })) if findings.is_empty() => Some(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph(look, "circle-check", Size::Small, look.color(token::RUN_PASSED)))
                    .child("Nothing found")
                    .into_any_element(),
            ),
            (ReviewRunState::Done, Some(ReviewResult::Findings { findings })) => Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .children(findings.iter().enumerate().map(|(at, finding)| finding_card(&key, at, finding, look)))
                    .into_any_element(),
            ),
            _ => None,
        };
        labelled(
            div()
                .id(SharedString::from(format!("pull-run:{key}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .p(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_RAISED))
                .child(head)
                .children(body),
            Role::Group,
            run.name.clone(),
        )
        .into_any_element()
    }
}

/// One finding: its severity chip, title and place, then what it says.
fn finding_card(run: &str, at: usize, finding: &ReviewFinding, look: &Look) -> AnyElement {
    let (icon, word, hue, said) = severity_look(finding.severity);
    let place = if finding.at.end > finding.at.start { format!("{}:{}-{}", finding.at.path, finding.at.start, finding.at.end) } else { format!("{}:{}", finding.at.path, finding.at.start) };
    let chip = labelled(state_chip(look, format!("pull-finding-severity:{run}:{at}"), icon, said.to_string(), word, hue), Role::Status, format!("{word} severity"));
    labelled(
        div()
            .id(SharedString::from(format!("pull-finding:{run}:{at}")))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .bg(look.color(token::BG_SUNKEN))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(chip)
                    .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(finding.title.clone()))),
            )
            .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).font_family(mono(look)).truncate().child(SharedString::from(place)))
            .child(shared::markdown(format!("pull-finding-body:{run}:{at}"), &finding.body, look)),
        Role::Article,
        finding.title.clone(),
    )
    .into_any_element()
}

/// What the mock Fleet answers a Start with: a finished run with two findings.
fn mock_run(manifest: &str, number: u64, head: &str, tool: &Pick) -> ReviewRun {
    let at = |path: &str, start: u64, end: u64| ipc::ReviewPlace { path: path.into(), side: ipc::DiffSide::Right, start, end };
    ReviewRun {
        manifest_id: ipc::ManifestId::carried(manifest),
        number,
        kind: ReviewRunKind::Tool,
        name: tool.id.clone(),
        head: head.into(),
        since: None,
        state: ReviewRunState::Done,
        started_at: "2026-11-20T10:00:00Z".into(),
        finished_at: Some("2026-11-20T10:01:00Z".into()),
        failed: None,
        result: Some(ReviewResult::Findings {
            findings: vec![
                ReviewFinding {
                    title: "The wait can pass its cap".into(),
                    body: "The jitter is added after the cap, so the last wait can run past it.".into(),
                    severity: ReviewSeverity::High,
                    at: at("crates/config/settings.toml", 12, 14),
                    comment: String::new(),
                },
                ReviewFinding {
                    title: "The new setting has no test".into(),
                    body: "Nothing reads the default back.".into(),
                    severity: ReviewSeverity::Low,
                    at: at("crates/config/settings.toml", 20, 20),
                    comment: String::new(),
                },
            ],
        }),
        cost_micros: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(name: &str, head: &str, at: &str, state: ReviewRunState) -> ReviewRun {
        ReviewRun {
            manifest_id: ipc::ManifestId::carried("mock-01"),
            number: 2209,
            kind: ReviewRunKind::Tool,
            name: name.into(),
            head: head.into(),
            since: None,
            state,
            started_at: at.into(),
            finished_at: None,
            failed: None,
            result: None,
            cost_micros: None,
        }
    }

    #[test]
    fn a_tool_shows_its_latest_run_at_the_head_and_none_from_another_head() {
        let runs = [
            run("bugs", "a1", "2026-11-20T10:00:00Z", ReviewRunState::Done),
            run("bugs", "a1", "2026-11-20T11:00:00Z", ReviewRunState::Running),
            run("risks", "old", "2026-11-20T09:00:00Z", ReviewRunState::Done),
        ];
        let shown = tool_runs(&runs, Some("a1"));
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].state, ReviewRunState::Running);
    }

    /// #2209 on mock-01, its Review tab in front, drawn wide.
    fn review_tab(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        use super::super::parts_fold::{redraw, rooted_job};
        let (view, cx) = rooted_job(cx, "job-pulls", "mock-01", gpui::size(gpui::px(2400.), gpui::px(1100.)), |_| {});
        view.update_in(cx, |view, window, cx| {
            let pull = view.pulls_of("mock-01", cx).into_iter().find(|pull| pull.number == 2209).expect("the Job holds #2209");
            view.open_pull(pull, window, cx);
            view.pull_tab(PullTab::Review, window, cx);
        });
        redraw(cx);
        (view, cx)
    }

    fn found(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().find(window.window_handle().window_id(), role, name).is_some())
    }

    #[gpui::test]
    fn the_review_tab_offers_the_repositorys_tools_and_a_start(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = review_tab(cx);
        assert!(found(cx, Role::RadioButton, "Possible bugs"), "a tool can be picked");
        assert!(found(cx, Role::RadioButton, "Risks"), "every tool Fleet lists is offered");
        assert!(found(cx, Role::Button, "Start"), "a run starts from a press");
        assert!(!found(cx, Role::Group, "bugs"), "nothing runs until Start");
    }

    #[gpui::test]
    fn start_runs_the_picked_tool_and_its_findings_are_read_in_the_tab(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::redraw;
        let (view, cx) = review_tab(cx);
        view.update_in(cx, |view, window, cx| {
            view.pick_pull_tool("risks".into(), cx);
            view.start_pull_tool(window, cx);
        });
        redraw(cx);
        assert!(found(cx, Role::Group, "risks"), "the run has its card");
        assert!(found(cx, Role::Status, "risks done"), "its state reads as a chip");
        assert!(found(cx, Role::Article, "The wait can pass its cap"), "a finding is read in place");
        assert!(found(cx, Role::Status, "High severity"), "its severity is a chip");
    }

    #[gpui::test]
    fn a_run_that_moves_on_fleets_stream_shows_running_then_its_findings(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::redraw;
        let (view, cx) = review_tab(cx);
        let running = run("bugs", "", "2026-11-20T10:00:00Z", ReviewRunState::Running);
        // The run is for the open pull request, whatever manifest and head the panel holds.
        let (manifest, number, head) = view.read_with(cx, |view, _| {
            let panel = view.pull_panel().expect("a pull request is open");
            (panel.pull.manifest.clone().expect("it has a manifest"), panel.pull.number, panel.page().and_then(|page| page.head.clone()).unwrap_or_default())
        });
        let moved = |run: ReviewRun| crate::fleet::ReviewRunsMoved { runs: vec![ReviewRun { manifest_id: ipc::ManifestId::carried(&manifest), number, head: head.clone(), ..run }], lost: false };
        view.update_in(cx, |view, window, cx| view.review_runs_moved(&moved(running), window, cx));
        redraw(cx);
        assert!(found(cx, Role::Status, "bugs running"), "a running run says so");
        let done = ReviewRun { state: ReviewRunState::Done, result: Some(ReviewResult::Findings { findings: Vec::new() }), ..run("bugs", "", "2026-11-20T10:00:00Z", ReviewRunState::Done) };
        view.update_in(cx, |view, window, cx| view.review_runs_moved(&moved(done), window, cx));
        redraw(cx);
        assert!(found(cx, Role::Status, "bugs done"), "the same run lands over itself");
        assert!(!found(cx, Role::Status, "bugs running"));
    }
}

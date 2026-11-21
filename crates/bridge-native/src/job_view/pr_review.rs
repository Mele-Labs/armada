//! A pull request panel's Review tab: what changed since its author last looked, the way the Reviews
//! screen says it for a reviewer. Who said what, the commits after the head he left it at, the files
//! they changed and the checks, from the page Fleet already read. `crate::since_looked` and the Reviews
//! model carry the range; nothing here asks Fleet for more than the compare and one commit's files.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{
    PullRequestCommit, PullRequestCompare, PullRequestEntry, PullRequestFilePatch, PullRequestPage,
    PullRequestVerdict,
};

use super::pr_panel::{
    mark, pull_checks, small_button, stand_ins, verdict_look, PullPanel, PullTab,
};
use super::rows::{mono, tip};
use super::JobView;
use crate::diff;
use crate::labels::labelled;
use crate::look::{token, Look};
use crate::reviews::model::Seen;
use crate::since_looked::{new_commits_of, seen_of};

/// One reviewer's words since he looked: the verdict they left last and how many comments they wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Heard {
    pub who: String,
    pub verdict: Option<PullRequestVerdict>,
    pub comments: usize,
}

/// What the Review tab says. Nothing is kept for a pull request he never marked seen.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Reading {
    pub seen: Option<Seen>,
    /// In the order each first spoke.
    pub heard: Vec<Heard>,
    /// Newest first, and none of his own.
    pub commits: Vec<PullRequestCommit>,
    /// How many of the commits since he looked he pushed himself, which the list leaves out.
    pub own_left_out: usize,
}

/// The range since he last looked, as Fleet's compare answered it, for the head he left it at and the one it is at.
#[derive(Clone, Debug)]
pub(super) struct Since {
    pub from: String,
    pub to: String,
    pub files: Vec<PullRequestFilePatch>,
    /// The forge no longer has the head he left it at.
    pub gone: bool,
}

impl JobView {
    /// The range since he looked, read once the tab is in front and again when his seen head or the head moves.
    /// An answer that failed stays, so a refusal is not asked for again and again.
    pub(super) fn want_pull_since(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.want_pull_tools(window, cx);
        let Some(panel) = self
            .pull_panel()
            .filter(|panel| panel.tab == PullTab::Review)
        else {
            return;
        };
        let (Some(seen), Some(head)) = (
            panel.page().and_then(|page| page.seen.clone()),
            panel.page().and_then(|page| page.head.clone()),
        ) else {
            return;
        };
        if seen.head == head {
            return;
        }
        let current = |since: &Result<Since, String>| {
            since
                .as_ref()
                .map_or(true, |since| since.from == seen.head && since.to == head)
        };
        if panel.since.as_ref().is_some_and(current) {
            return;
        }
        let (Some(manifest), number) = (panel.pull.manifest.clone(), panel.pull.number) else {
            return;
        };
        let path = format!(
            "/pull_request_compares/{}/{number}?since={}&head={}",
            super::wire::encoded(&manifest),
            super::wire::encoded(&seen.head),
            super::wire::encoded(&head)
        );
        let (from, to) = (seen.head, head);
        let made = move |compare: PullRequestCompare| match compare {
            PullRequestCompare::Patches { files, .. } => Since {
                from: from.clone(),
                to: to.clone(),
                files,
                gone: false,
            },
            PullRequestCompare::BaseGone { .. } => Since {
                from: from.clone(),
                to: to.clone(),
                files: Vec::new(),
                gone: true,
            },
        };
        if self.fleet.is_none() {
            let read = crate::mock::pull_compare(&format!("{manifest}#{number}"))
                .map(made)
                .ok_or_else(|| "The forge did not answer for this range".to_string());
            if let Some(panel) = self.pull_panel_mut() {
                panel.since = Some(read);
            }
            cx.notify();
            return;
        }
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let read = answer
                    .and_then(|bytes| {
                        ipc::decode::<PullRequestCompare>("PullRequestCompare", &bytes)
                            .map_err(|why| why.to_string())
                    })
                    .map(made);
                if let Some(panel) = this
                    .pull_panel_mut()
                    .filter(|panel| panel.pull.number == number)
                {
                    panel.since = Some(read);
                }
                cx.notify();
            }),
        );
    }

    /// Diff since you looked: the files the range changed are drawn under the new commits, or put away again.
    pub(super) fn pull_since_toggle(&mut self, cx: &mut Context<Self>) {
        if let Some(panel) = self.pull_panel_mut() {
            panel.since_shown = !panel.since_shown;
        }
        cx.notify();
    }

    /// Mark as seen (`POST /pull_request_seen/:repository`): Fleet keeps the head he is looking at, and the page says so.
    pub(super) fn mark_pull_seen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel_mut().filter(|panel| !panel.marking) else {
            return;
        };
        let (Some(manifest), Some(head)) = (
            panel.pull.manifest.clone(),
            panel.page().and_then(|page| page.head.clone()),
        ) else {
            return;
        };
        let number = panel.pull.number;
        panel.marking = true;
        let local = self.fleet.is_none();
        self.post(
            format!("/pull_request_seen/{}", super::wire::encoded(&manifest)),
            ipc::encode(&ipc::MarkPullSeen {
                number,
                head: head.clone(),
            }),
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                let Some(panel) = this
                    .pull_panel_mut()
                    .filter(|panel| panel.pull.number == number)
                else {
                    return;
                };
                panel.marking = false;
                // Without a Fleet the mock keeps it here, as now.
                let seen = answer
                    .ok()
                    .and_then(|bytes| ipc::decode::<ipc::PullSeen>("PullSeen", &bytes).ok())
                    .or_else(|| {
                        local.then(|| ipc::PullSeen {
                            head,
                            at: chrono::Utc::now()
                                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        })
                    });
                if let (Some(seen), Some(Ok(page))) = (seen, panel.page.as_mut()) {
                    page.seen = Some(seen);
                    panel.since = None;
                    panel.since_shown = false;
                }
                cx.notify();
            }),
        );
    }

    /// The Review tab: where he left the pull request, who has spoken since, the commits after his and what
    /// they changed, and the checks. Nothing to read, nothing drawn.
    pub(super) fn pull_review(
        &self,
        panel: &PullPanel,
        page: &PullRequestPage,
        look: &Look,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        let gone = panel
            .since
            .as_ref()
            .is_some_and(|since| since.as_ref().is_ok_and(|since| since.gone));
        let reading = reading_of(page, gone);
        let (muted, size) = (look.color(token::FG_MUTED), look.length(token::TEXT_XS));
        let heading = |text: &str| {
            div()
                .text_size(look.length(token::TEXT_SM))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .child(SharedString::from(text.to_string()))
        };
        let moved = reading
            .seen
            .as_ref()
            .is_some_and(|seen| seen.old != seen.new);
        let heard_any = !reading.heard.is_empty();
        let said = match &reading.seen {
            None => "You have not marked this pull request seen, so nothing is told apart as new.".to_string(),
            Some(seen) if seen.gone => format!("The commit you looked at, {}, is gone after a force push: every commit counts as new.", seen.old),
            Some(seen) if !moved && !heard_any => format!("Nothing has changed since you looked {}.", seen.when()),
            Some(seen) => format!("{} \u{2192} {}", seen.old, seen.new),
        };
        let title = match &reading.seen {
            Some(seen) => format!("Since you looked {}", seen.when()),
            None => "Not looked at yet".to_string(),
        };
        let seen_name = if reading.seen.is_some() {
            "Since you looked"
        } else {
            "Not looked at yet"
        };
        let ask = reading.seen.is_none() || moved || heard_any;
        let mark_seen = labelled(
            small_button(
                look,
                "pull-review-seen",
                "Mark as seen",
                ask,
                !panel.marking,
            )
            .tooltip(tip(
                "Keeps the head you are looking at, so what comes after it reads as new",
            ))
            .on_click(cx.listener(|this, _, window, cx| this.mark_pull_seen(window, cx))),
            Role::Button,
            "Mark as seen",
        );
        let diff_button =
            (moved && !reading.seen.as_ref().is_some_and(|seen| seen.gone)).then(|| {
                let said = if panel.since_shown {
                    "Hide the diff"
                } else {
                    "Diff since you looked"
                };
                labelled(
                    small_button(look, "pull-review-diff", said, false, true)
                        .on_click(cx.listener(|this, _, _, cx| this.pull_since_toggle(cx))),
                    Role::Button,
                    said,
                )
            });
        let head = labelled(
            div()
                .id("pull-review-head")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(heading(&title))
                .child(
                    div()
                        .text_size(size)
                        .text_color(muted)
                        .font_family(mono(look))
                        .child(SharedString::from(said)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .child(mark_seen)
                        .children(diff_button),
                ),
            Role::Region,
            seen_name,
        );
        let heard = reading.seen.is_some().then(|| {
            if reading.heard.is_empty() {
                return div()
                    .text_size(size)
                    .text_color(muted)
                    .child("Nobody has said anything since.")
                    .into_any_element();
            }
            let rows: Vec<AnyElement> = reading
                .heard
                .iter()
                .enumerate()
                .map(|(at, one)| {
                    let (icon, verdict, tone) = verdict_look(one.verdict);
                    let comments = match one.comments {
                        0 => None,
                        1 => Some("1 comment".to_string()),
                        n => Some(format!("{n} comments")),
                    };
                    let note = [one.verdict.map(|_| verdict.to_string()), comments]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>()
                        .join(", ");
                    labelled(
                        div()
                            .id(SharedString::from(format!("pull-review-heard:{at}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .py(look.length(token::SPACE_2))
                            .text_size(size)
                            .text_color(muted)
                            .child(mark(
                                look,
                                format!("pull-review-verdict:{at}"),
                                icon,
                                verdict,
                                tone,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(SharedString::from(one.who.clone())),
                            )
                            .child(div().flex_none().child(SharedString::from(note))),
                        Role::ListItem,
                        format!("{} spoke since you looked", one.who),
                    )
                    .into_any_element()
                })
                .collect();
            labelled(
                div()
                    .id("pull-review-heard")
                    .flex()
                    .flex_col()
                    .children(rows),
                Role::List,
                "Heard since you looked",
            )
            .into_any_element()
        });
        let commits = (!reading.commits.is_empty()).then(|| {
            let rows: Vec<AnyElement> = reading
                .commits
                .iter()
                .enumerate()
                .map(|(at, commit)| {
                    let (sha, headline) = (commit.sha.clone(), commit.headline.clone());
                    let hover = look.color(token::BG_HOVER);
                    labelled(
                        div()
                            .id(SharedString::from(format!("pull-review-commit:{at}")))
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_3))
                            .py(look.length(token::SPACE_2))
                            .cursor_pointer()
                            .hover(move |style| style.bg(hover))
                            .tooltip(tip("Shows the files it changed"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.show_commit(sha.clone(), headline.clone(), window, cx)
                            }))
                            .text_size(size)
                            .text_color(muted)
                            .child(div().flex_none().font_family(mono(look)).child(
                                SharedString::from(commit.sha.chars().take(7).collect::<String>()),
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_color(look.color(token::FG_DEFAULT))
                                    .child(SharedString::from(commit.headline.clone())),
                            )
                            .children(
                                commit.author.clone().map(|author| {
                                    div().flex_none().child(SharedString::from(author))
                                }),
                            )
                            .children(
                                commit
                                    .at
                                    .clone()
                                    .map(|at| div().flex_none().child(SharedString::from(at))),
                            ),
                        Role::ListItem,
                        commit.headline.clone(),
                    )
                    .into_any_element()
                })
                .collect();
            labelled(
                div()
                    .id("pull-review-commits")
                    .flex()
                    .flex_col()
                    .children(rows),
                Role::List,
                "New commits",
            )
            .into_any_element()
        });
        let own = (reading.own_left_out > 0).then(|| {
            let said = if reading.own_left_out == 1 {
                "1 of your own pushes is left out.".to_string()
            } else {
                format!("{} of your own pushes are left out.", reading.own_left_out)
            };
            div()
                .text_size(size)
                .text_color(muted)
                .child(SharedString::from(said))
        });
        let range = panel.since_shown.then(|| match panel.since.as_ref() {
            None => stand_ins(look, "pull-review-reading", &[0.9, 0.7, 0.8]),
            Some(Err(why)) => div()
                .text_size(size)
                .text_color(muted)
                .child(SharedString::from(why.clone()))
                .into_any_element(),
            Some(Ok(since)) => {
                let files: Vec<AnyElement> = since
                    .files
                    .iter()
                    .enumerate()
                    .map(|(at, file)| {
                        let drawn = diff::file_diff_of(
                            &file.path,
                            file.patch.as_deref(),
                            file.said.as_deref(),
                        );
                        let body = match drawn.files.first() {
                            Some(lines) => {
                                diff::lines(format!("pull-review-lines:{at}"), &lines.lines, look)
                                    .into_any_element()
                            }
                            None => div()
                                .p(look.length(token::SPACE_3))
                                .text_size(size)
                                .text_color(muted)
                                .child(SharedString::from(drawn.empty.clone().unwrap_or_default()))
                                .into_any_element(),
                        };
                        labelled(
                            div()
                                .id(SharedString::from(format!("pull-review-file:{at}")))
                                .flex()
                                .flex_col()
                                .min_w_0()
                                .overflow_hidden()
                                .rounded(look.length(token::RADIUS_SM))
                                .border(look.length(token::BORDER_WIDTH))
                                .border_color(look.color(token::BORDER_SUBTLE))
                                .child(
                                    div()
                                        .px(look.length(token::SPACE_3))
                                        .py(look.length(token::SPACE_2))
                                        .bg(look.color(token::BG_RAISED))
                                        .border_b(look.length(token::BORDER_WIDTH))
                                        .border_color(look.color(token::BORDER_SUBTLE))
                                        .truncate()
                                        .font_family(mono(look))
                                        .text_size(size)
                                        .child(SharedString::from(file.path.clone())),
                                )
                                .child(body),
                            Role::Group,
                            file.path.clone(),
                        )
                        .into_any_element()
                    })
                    .collect();
                div()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .children(files)
                    .into_any_element()
            }
        });
        let section = |title: &str, body: AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .child(heading(title))
                .child(body)
                .into_any_element()
        };
        let checks = pull_checks(page, look, cx).map(|checks| section("Checks", checks));
        let column = div()
            .id("pull-review")
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_4))
            .child(section("Review tools", self.pull_tools(panel, look, cx)))
            .child(head)
            .children(heard.map(|heard| section("Heard since you looked", heard)))
            .children(commits.map(|commits| {
                section(
                    "New commits",
                    div()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_1))
                        .child(commits)
                        .children(own)
                        .into_any_element(),
                )
            }))
            .children(range.map(|range| section("Files changed since you looked", range)))
            .children(checks);
        Some(labelled(column, Role::List, "Review").into_any_element())
    }
}

/// The reading of a page, with `gone` where the forge no longer has the head he left it at.
pub(super) fn reading_of(page: &PullRequestPage, gone: bool) -> Reading {
    let Some(left_at) = page
        .seen
        .as_ref()
        .and_then(|seen| chrono::DateTime::parse_from_rfc3339(&seen.at).ok())
    else {
        return Reading {
            seen: seen_of(page, gone),
            ..Reading::default()
        };
    };
    let author = page.author.as_deref().unwrap_or_default();
    let mine = |who: &str| who.eq_ignore_ascii_case(author);
    let after = |at: &str| chrono::DateTime::parse_from_rfc3339(at).is_ok_and(|at| at > left_at);
    let mut heard: Vec<Heard> = Vec::new();
    let mut hear = |who: &str, verdict: Option<PullRequestVerdict>, comment: bool| {
        let at = match heard.iter().position(|one| one.who == who) {
            Some(at) => at,
            None => {
                heard.push(Heard {
                    who: who.to_string(),
                    verdict: None,
                    comments: 0,
                });
                heard.len() - 1
            }
        };
        heard[at].verdict = verdict.or(heard[at].verdict);
        heard[at].comments += usize::from(comment);
    };
    for entry in &page.conversation {
        match entry {
            PullRequestEntry::Comment {
                author,
                bot: false,
                at,
                ..
            } if after(at) && !mine(author) => hear(author, None, true),
            PullRequestEntry::Review {
                author,
                bot: false,
                at,
                verdict,
                ..
            } if after(at) && !mine(author) => hear(author, Some(*verdict), false),
            _ => {}
        }
    }
    for comment in page.threads.iter().flat_map(|thread| &thread.comments) {
        if !comment.bot && after(&comment.at) && !mine(&comment.author) {
            hear(&comment.author, None, true);
        }
    }
    let (own, others): (Vec<PullRequestCommit>, Vec<PullRequestCommit>) = new_commits_of(page)
        .into_iter()
        .partition(|commit| commit.author.as_deref().is_some_and(mine));
    Reading {
        seen: seen_of(page, gone),
        heard,
        commits: others,
        own_left_out: own.len(),
    }
}

#[cfg(test)]
mod tests {
    use ipc::{PullReviewThread, PullReviewThreadComment, PullSeen};

    use super::*;

    fn comment(by: &str, at: &str, bot: bool) -> PullRequestEntry {
        PullRequestEntry::Comment {
            id: format!("c-{by}-{at}"),
            author: by.into(),
            bot,
            at: at.into(),
            edited_at: None,
            body: "A word.".into(),
        }
    }

    fn review(by: &str, at: &str, verdict: PullRequestVerdict) -> PullRequestEntry {
        PullRequestEntry::Review {
            id: format!("r-{by}-{at}"),
            author: by.into(),
            bot: false,
            at: at.into(),
            edited_at: None,
            verdict,
            body: String::new(),
        }
    }

    fn commit(sha: &str, by: &str) -> PullRequestCommit {
        PullRequestCommit {
            sha: sha.into(),
            headline: format!("Commit {sha}"),
            author: Some(by.into()),
            at: None,
        }
    }

    /// Mira's pull request, which she last left at `bbbbbbb` at 10:30, with her own push and Devon's after it.
    fn page() -> PullRequestPage {
        let thread = PullReviewThread {
            id: "t1".into(),
            path: "src/page.rs".into(),
            line: Some(42),
            side: ipc::DiffSide::Right,
            resolved: false,
            comments: vec![PullReviewThreadComment {
                id: "tc1".into(),
                author: "devon".into(),
                bot: false,
                at: "2026-11-08T11:30:00Z".into(),
                edited_at: None,
                body: "Panics on an empty page.".into(),
            }],
        };
        PullRequestPage {
            author: Some("mira".into()),
            head: Some("dddddddffffffffffffffffffffffffffffffff".into()),
            seen: Some(PullSeen {
                head: "bbbbbbb1111111111111111111111111111111".into(),
                at: "2026-11-08T10:30:00Z".into(),
            }),
            commits: vec![
                commit("aaaaaaa", "mira"),
                commit("bbbbbbb", "mira"),
                commit("ccccccc", "mira"),
                commit("ddddddd", "devon"),
            ],
            conversation: vec![
                comment("devon", "2026-11-08T10:00:00Z", false),
                comment("devon", "2026-11-08T10:50:00Z", false),
                comment("mira", "2026-11-08T11:08:00Z", false),
                comment("lint-bot", "2026-11-08T11:10:00Z", true),
                review(
                    "devon",
                    "2026-11-08T11:20:00Z",
                    PullRequestVerdict::ChangesRequested,
                ),
                review("noor", "2026-11-08T12:00:00Z", PullRequestVerdict::Approved),
            ],
            threads: vec![thread],
            ..PullRequestPage::default()
        }
    }

    #[test]
    fn a_reviewer_who_spoke_since_he_looked_is_named_with_the_verdict_they_left_and_their_comments()
    {
        let heard = reading_of(&page(), false).heard;
        assert_eq!(
            heard,
            vec![
                Heard {
                    who: "devon".into(),
                    verdict: Some(PullRequestVerdict::ChangesRequested),
                    comments: 2
                },
                Heard {
                    who: "noor".into(),
                    verdict: Some(PullRequestVerdict::Approved),
                    comments: 0
                },
            ],
            "what Devon said before he looked is not counted, and the thread comment is"
        );
    }

    #[test]
    fn his_own_words_and_an_apps_are_not_heard() {
        let heard = reading_of(&page(), false).heard;
        assert!(
            heard
                .iter()
                .all(|one| one.who != "mira" && one.who != "lint-bot"),
            "{heard:?}"
        );
    }

    #[test]
    fn his_own_pushes_are_left_out_of_the_new_commits_and_counted() {
        let reading = reading_of(&page(), false);
        let shas: Vec<&str> = reading.commits.iter().map(|one| one.sha.as_str()).collect();
        assert_eq!(shas, ["ddddddd"]);
        assert_eq!(reading.own_left_out, 1);
    }

    #[test]
    fn a_pull_request_he_never_looked_at_reads_as_nothing_since() {
        let reading = reading_of(
            &PullRequestPage {
                seen: None,
                ..page()
            },
            false,
        );
        assert_eq!(reading, Reading::default());
    }

    #[test]
    fn a_head_the_forge_no_longer_has_is_said_so() {
        let reading = reading_of(&page(), true);
        assert!(reading.seen.is_some_and(|seen| seen.gone));
    }

    /// #2209 on mock-01, its Review tab in front, drawn wide.
    fn review_tab(
        cx: &mut gpui::TestAppContext,
    ) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        use super::super::parts_fold::{redraw, rooted_job};
        let (view, cx) = rooted_job(
            cx,
            "job-pulls",
            "mock-01",
            gpui::size(gpui::px(2400.), gpui::px(1100.)),
            |_| {},
        );
        view.update_in(cx, |view, window, cx| {
            let pull = view
                .pulls_of("mock-01", cx)
                .into_iter()
                .find(|pull| pull.number == 2209)
                .expect("the Job holds #2209");
            view.open_pull(pull, window, cx);
            view.pull_tab(super::super::pr_panel::PullTab::Review, window, cx);
        });
        redraw(cx);
        (view, cx)
    }

    fn found(
        cx: &mut gpui::VisualTestContext,
        role: gpui::Role,
        name: &str,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        cx.update(|window, cx| {
            cx.global::<crate::labels::LabelBook>()
                .find(window.window_handle().window_id(), role, name)
                .map(|one| one.bounds)
        })
    }

    #[gpui::test]
    fn the_review_tab_says_when_he_looked_and_who_spoke_since(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = review_tab(cx);
        assert!(
            found(cx, gpui::Role::Region, "Since you looked").is_some(),
            "the tab says where he left it"
        );
        for who in ["marta", "nina"] {
            assert!(
                found(
                    cx,
                    gpui::Role::ListItem,
                    &format!("{who} spoke since you looked")
                )
                .is_some(),
                "{who} spoke after he looked"
            );
        }
        assert!(
            found(cx, gpui::Role::ListItem, "nick spoke since you looked").is_none(),
            "his own words are not heard"
        );
    }

    #[gpui::test]
    fn the_review_tab_lists_the_commits_after_his_but_not_his_own(cx: &mut gpui::TestAppContext) {
        let (_view, cx) = review_tab(cx);
        assert!(
            found(
                cx,
                gpui::Role::ListItem,
                "Keep Open in browser on the panel"
            )
            .is_some(),
            "the Job's push is new to him"
        );
        assert!(
            found(
                cx,
                gpui::Role::ListItem,
                "Add a default for links.openPullRequestsIn"
            )
            .is_none(),
            "his own push is left out"
        );
        assert!(
            found(cx, gpui::Role::ListItem, "Read the setting once, in main").is_none(),
            "the head he left it at is not new"
        );
    }

    #[gpui::test]
    fn diff_since_you_looked_draws_the_files_of_the_range(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::redraw;
        let (view, cx) = review_tab(cx);
        assert!(
            found(cx, gpui::Role::Group, "crates/config/settings.toml").is_none(),
            "the diff waits to be asked for"
        );
        view.update_in(cx, |view, _, cx| view.pull_since_toggle(cx));
        redraw(cx);
        assert!(
            found(cx, gpui::Role::Group, "crates/config/settings.toml").is_some(),
            "the files changed since he looked are drawn"
        );
    }

    #[gpui::test]
    fn mark_as_seen_leaves_nothing_new(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::redraw;
        let (view, cx) = review_tab(cx);
        view.update_in(cx, |view, window, cx| view.mark_pull_seen(window, cx));
        redraw(cx);
        assert!(
            found(cx, gpui::Role::List, "New commits").is_none(),
            "he has seen the head"
        );
        assert!(
            found(cx, gpui::Role::ListItem, "marta spoke since you looked").is_none(),
            "nobody has spoken since he marked it"
        );
        assert!(
            found(cx, gpui::Role::Region, "Since you looked").is_some(),
            "the tab still says where he left it"
        );
    }
}

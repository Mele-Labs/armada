//! A finished Job's work, brought up to its base and put where a person
//! reviews it.
//!
//! **The base is merged in, never rebased onto** — `#1131`. One pass meets
//! every conflict, reviewed commits keep their ids, and a plain push carries it;
//! `crate::merging_in` is how. Every catch-up here comes before a Drone is put
//! on the worktree, so a conflict's markers stay for that Drone to clear.
//!
//! **The base is the remote's copy of it**, not the local branch of the same
//! name: that one is checked out in a person's own checkout and is rarely
//! fast-forwarded, so a catch-up that merged it left a pull request conflicting
//! while reporting the branch up to date (Job 14, 12 Oct 2026). Nothing here
//! fetches: [`Delivery::fetch_base`] is its own call, so a failure to reach the
//! remote is seen by the caller and not buried in a step boundary. Where there
//! is no remote copy — no remote, or a base never pushed — the local branch is
//! the only reading there is. `crate::fetching_the_base`.

use std::process::{Command, Output};

use adapter_traits::{
    Base, BaseFetched, BaseOnTheRemote, BranchMerged, BroughtUpToDate, Delivery, KeptCurrent, Landable, Merged,
    NotDelivered, NotMerged, Opened, Pushed, PushedOntoBase, RepositoryStanding, Review,
    UncheckedHead, UnderReview, WhatBecameOfIt,
};
use adapter_traits::{Standing, Worktree, WorktreeSpec};
use git2::{BranchType, Repository};

use crate::fetching_the_base;
use crate::merging_in::{self, MergedIn, OnConflict};
use crate::worktree::GitVcs;

/// The command that opens a pull request, and later says what became of it.
/// **The one vendor name in this crate**, shared with `crate::landing`, and the
/// reason the whole of delivery lives in `adapters` at all.
pub(crate) const FORGE: &str = "gh";

impl Delivery for GitVcs {
    fn base(
        &self,
        worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<Option<Base>, NotDelivered> {
        crate::base::resolve(&open(worktree)?, declared)
    }

    fn standing(&self, worktree: &Worktree, base: &Base) -> Result<Standing, NotDelivered> {
        let repo = open(worktree)?;
        let tip = head_of(&repo, worktree)?;
        let base_tip = the_base_tip(&repo, base.name())?;
        // A merge part-way through stands where it is merging to once its
        // markers are cleared, so the next Drone is not told of it again.
        // Until then it reads behind, so the Drone put on it is.
        let from = match merging_in::merge_head(&repo) {
            Some(merging) if merging_in::still_marked(worktree).is_empty() => merging,
            _ => tip,
        };
        match repo.graph_ahead_behind(from, base_tip) {
            Ok((_, 0)) => Ok(Standing::UpToDate),
            Ok((_, behind)) => Ok(Standing::Behind { commits: behind }),
            Err(cause) => Err(NotDelivered::of(
                "comparing the branch with its base",
                cause.message(),
            )),
        }
    }

    fn ahead(&self, worktree: &Worktree, base: &Base) -> Result<usize, NotDelivered> {
        let repo = open(worktree)?;
        let tip = head_of(&repo, worktree)?;
        let base_tip = the_base_tip(&repo, base.name())?;
        repo.graph_ahead_behind(tip, base_tip)
            .map(|(ahead, _)| ahead)
            .map_err(|cause| {
                NotDelivered::of("comparing the branch with its base", cause.message())
            })
    }

    fn base_on_the_remote(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BaseOnTheRemote, NotDelivered> {
        let repo = open(worktree)?;
        // **A base with no upstream is `Agreed`, not a refusal.** A repository
        // with no remote is ordinary here for the same reason `Pushed::NoRemote`
        // is, and so is a base branch nobody has ever pushed: there is no second
        // reading, so there is nothing to say about the two disagreeing.
        let Some((remote, remote_tip)) = the_upstream_of(&repo, base.name()) else {
            return Ok(BaseOnTheRemote::Agreed);
        };
        let local_tip = tip_of(&repo, base.name())?;
        match repo.graph_ahead_behind(local_tip, remote_tip) {
            Ok((0, 0)) => Ok(BaseOnTheRemote::Agreed),
            Ok((ahead, behind)) => Ok(BaseOnTheRemote::Apart {
                remote,
                ahead,
                behind,
            }),
            Err(cause) => Err(NotDelivered::of(
                "comparing the base with the remote's",
                cause.message(),
            )),
        }
    }

    fn fetch_base(&self, in_repo: &str, base: &str) -> BaseFetched {
        fetching_the_base::fetched(in_repo, base)
    }

    fn bring_up_to_date(
        &self,
        worktree: &Worktree,
        base: &Base,
    ) -> Result<BroughtUpToDate, NotDelivered> {
        let base_name = base.name().to_string();
        let onto = fetching_the_base::base_ref(&open(worktree)?, base.name());
        let merged = merging_in::merged_in(worktree, &onto, OnConflict::LeaveTheMarkers)?;
        Ok(match merged {
            MergedIn::Clean { commits } => BroughtUpToDate::Clean {
                base: base_name,
                commits,
            },
            MergedIn::Conflicted { files } => BroughtUpToDate::Conflicted {
                base: base_name,
                files,
            },
            MergedIn::PutBack { files } => BroughtUpToDate::PutBack {
                base: base_name,
                files,
            },
        })
    }

    fn merge_branch(
        &self,
        worktree: &Worktree,
        branch: &str,
    ) -> Result<BranchMerged, NotDelivered> {
        Ok(
            match merging_in::merged_in(worktree, branch, OnConflict::PutItBack)? {
                MergedIn::Clean { .. } => BranchMerged::Merged,
                MergedIn::Conflicted { files } | MergedIn::PutBack { files } => {
                    BranchMerged::PutBack { files }
                }
            },
        )
    }

    fn push(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered> {
        pushed(worktree)
    }

    fn push_forcing(&self, worktree: &Worktree) -> Result<Pushed, NotDelivered> {
        crate::keeping_current::pushed_forcing(worktree)
    }

    fn open_for_review(
        &self,
        worktree: &Worktree,
        base: &Base,
        review: &Review,
    ) -> Result<Opened, NotDelivered> {
        // Asked before it is created, because creating a second one for a
        // branch is refused with a sentence rather than answered with the first.
        if let Some(url) = already_open(worktree) {
            return Ok(Opened::AlreadyOpen { url });
        }
        let mut args = vec![
            "pr",
            "create",
            "--base",
            base.name(),
            "--head",
            worktree.branch(),
            "--title",
            review.title(),
            "--body",
            review.body(),
        ];
        if review.draft() {
            args.push("--draft");
        }
        let run = run(worktree, FORGE, &args);
        let run = match run {
            Ok(run) => run,
            Err(why) => {
                return Ok(Opened::NoTool {
                    why: format!("`{FORGE}` would not run: {why}"),
                })
            }
        };
        match run.status.success() {
            true => Ok(Opened::PullRequest {
                url: last_line(&run),
            }),
            false => Err(NotDelivered::of("the pull request", said(&run))),
        }
    }

    fn landed(&self, in_repo: &str, pull_request: &str) -> WhatBecameOfIt {
        crate::landing::read(in_repo, pull_request)
    }

    fn under_review(&self, in_repo: &str, pull_request: &str, gate: &str) -> UnderReview {
        crate::under_review::read(in_repo, pull_request, gate)
    }

    fn inline_remarks(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::Remark>> {
        crate::inline_comments::read(in_repo, pull_request)
    }

    fn base_head_on_the_forge(&self, in_repo: &str, base: &str) -> Option<String> {
        crate::main_ci::head(in_repo, base)
    }

    fn ci_runs_on(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::CiRuns> {
        crate::main_ci::runs(in_repo, commit)
    }

    fn queue_commit_of(&self, in_repo: &str, base: &str, number: u64) -> Option<String> {
        crate::main_ci::queue_commit_of(in_repo, base, number)
    }

    fn ci_log(
        &self,
        in_repo: &str,
        run: &adapter_traits::CiRun,
    ) -> Option<adapter_traits::FromOutside> {
        crate::main_ci::log(in_repo, run)
    }

    fn merged_by(&self, in_repo: &str, commit: &str) -> Option<adapter_traits::MergedPull> {
        crate::main_ci::merged_by(in_repo, commit)
    }

    fn merge_queue(&self, in_repo: &str, base: &str) -> Option<adapter_traits::MergeQueue> {
        crate::merge_queue::read(in_repo, base)
    }

    fn open_pull_requests(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::OpenPulls> {
        crate::main_ci::open_pulls(in_repo, gate)
    }

    fn open_pull_request(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::OpenPull> {
        crate::main_ci::open_pull(in_repo, gate, number)
    }

    fn pull_watch(&self, in_repo: &str, gate: &str) -> Option<adapter_traits::WatchedPulls> {
        crate::pull_watch::read(in_repo, gate)
    }

    fn watched_pull(
        &self,
        in_repo: &str,
        gate: &str,
        number: u64,
    ) -> Option<adapter_traits::WatchedPull> {
        crate::pull_watch::read_one(in_repo, gate, number)
    }

    fn forge_login(&self, in_repo: &str) -> Option<String> {
        crate::main_ci::forge_login(in_repo)
    }

    fn recently_merged_pull_requests(
        &self,
        in_repo: &str,
        base: &str,
        limit: usize,
    ) -> Option<adapter_traits::RecentlyMergedPulls> {
        crate::main_ci::recently_merged(in_repo, base, limit)
    }

    fn pull_request_diff(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestDiff> {
        crate::pull_request_diff::read(in_repo, pull_request)
    }

    fn rerun_failed(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Result<adapter_traits::Rerun, adapter_traits::NotRerun> {
        crate::rerunning::rerun_failed(in_repo, pull_request)
    }

    fn file_issue(
        &self,
        in_repo: &str,
        title: &str,
        body: &str,
    ) -> Result<adapter_traits::FiledIssue, adapter_traits::NotFiled> {
        crate::filing::file_issue(in_repo, title, body)
    }

    fn kept_current(&self, spec: &WorktreeSpec, base: &str) -> KeptCurrent {
        crate::keeping_current::kept_current(spec, base)
    }

    fn base_tip(&self, in_repo: &str, base: &str) -> Option<String> {
        crate::keeping_current::base_tip(in_repo, base)
    }

    fn merge(&self, in_repo: &str, pull_request: &str) -> Result<Merged, NotMerged> {
        crate::landing::merge(in_repo, pull_request)
    }

    fn enable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        crate::landing::enable_auto_merge(in_repo, pull_request)
    }

    fn disable_auto_merge(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        crate::landing::disable_auto_merge(in_repo, pull_request)
    }

    fn pull_request_facts(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestFacts> {
        crate::pull_request_facts::read(in_repo, pull_request)
    }

    fn mark_ready(&self, in_repo: &str, pull_request: &str) -> Result<(), String> {
        crate::pull_request_facts::mark_ready(in_repo, pull_request)
    }

    fn pull_request_page(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<adapter_traits::PullRequestPageFacts> {
        crate::pull_request_detail::read(in_repo, pull_request)
    }

    fn pull_request_file_patch(
        &self,
        in_repo: &str,
        pull_request: &str,
        path: &str,
    ) -> Option<adapter_traits::FilePatch> {
        crate::pull_request_patch::read(in_repo, pull_request, path)
    }

    fn pull_request_patches(
        &self,
        in_repo: &str,
        pull_request: &str,
    ) -> Option<Vec<adapter_traits::PatchedFile>> {
        crate::pull_request_patch::read_every(in_repo, pull_request)
    }

    fn write_pull_request(
        &self,
        in_repo: &str,
        pull_request: &str,
        write: &adapter_traits::PullRequestWrite,
    ) -> Result<(), String> {
        crate::pull_request_writes::write(in_repo, pull_request, write)
    }

    fn pull_review_list(
        &self,
        in_repo: &str,
        ask: &adapter_traits::ReviewListAsk<'_>,
    ) -> Option<Vec<adapter_traits::ListedPull>> {
        crate::pull_review_lists::read(in_repo, ask)
    }

    fn resolve_review_thread(
        &self,
        in_repo: &str,
        thread: &str,
        resolved: bool,
    ) -> Result<bool, String> {
        crate::pull_request_threads::resolve(in_repo, thread, resolved)
    }

    fn merge_by_push(
        &self,
        in_repo: &str,
        handle: &str,
        declared: Option<&str>,
        pull_request: Option<u64>,
        landable: Landable<'_>,
    ) -> Result<PushedOntoBase, NotMerged> {
        crate::landing::merge_by_push(in_repo, handle, declared, pull_request, landable)
    }

    fn tree_as_it_stands(&self, worktree: &Worktree) -> Result<String, NotDelivered> {
        crate::snapshot::tree_as_it_stands(std::path::Path::new(worktree.path()))
            .map_err(|why| NotDelivered::of("reading the worktree as a tree", why.to_string()))
    }

    fn the_unchecked_head(
        &self,
        in_repo: &str,
        worktree: &Worktree,
        declared: Option<&str>,
        checked: Option<&str>,
    ) -> Result<UncheckedHead, NotMerged> {
        crate::merging_the_base_in::the_unchecked_head(in_repo, worktree, declared, checked)
    }

    fn merge_the_moved_base_in(
        &self,
        in_repo: &str,
        worktree: &Worktree,
        declared: Option<&str>,
    ) -> Result<adapter_traits::BaseMergedIn, NotMerged> {
        crate::merging_the_base_in::merge_the_moved_base_in(in_repo, worktree, declared)
    }

    fn put_back(
        &self,
        worktree: &Worktree,
        merged: &adapter_traits::BaseMergedIn,
    ) -> Result<(), NotDelivered> {
        crate::merging_the_base_in::put_back(worktree, merged)
    }

    fn settle_worktree(&self, worktree: &Worktree) -> Result<(), NotDelivered> {
        crate::merging_the_base_in::settle_worktree(worktree)
    }

    fn caught_the_repository_up(&self, in_repo: &str, base: &str) -> RepositoryStanding {
        crate::landing::caught_up(in_repo, base)
    }

    fn checkout_has(&self, in_repo: &str, base: &str, commit: &str) -> bool {
        crate::landing::checkout_has(in_repo, base, commit)
    }

    fn base_fast_forwarded(&self, in_repo: &str, base: &str) -> bool {
        crate::landing::base_fast_forwarded(in_repo, base)
    }

    fn held_back(&self, in_repo: &str, base: &str) -> Option<adapter_traits::HeldBack> {
        crate::landing::held_back(in_repo, base)
    }

    fn pulled_the_checkout(&self, in_repo: &str) -> adapter_traits::CheckoutPulled {
        crate::landing::pulled_the_checkout(in_repo)
    }

    fn uncommitted_change_of(&self, in_repo: &str, path: &str) -> Result<String, String> {
        crate::landing::uncommitted_change_of(in_repo, path)
    }

    fn manifest_by_pull_request(&self, in_repo: &str, file: &str) -> bool {
        crate::manifest_edit::goes_by_pull_request(in_repo, file)
    }

    fn manifest_on_the_base(
        &self,
        in_repo: &str,
        base: &str,
        file: &str,
    ) -> Result<adapter_traits::ManifestOnTheBase, NotDelivered> {
        crate::manifest_edit::on_the_base(in_repo, base, file)
    }

    fn send_manifest_edit(
        &self,
        in_repo: &str,
        sending: &adapter_traits::ManifestEditToSend,
    ) -> Result<adapter_traits::ManifestEditSent, NotDelivered> {
        crate::manifest_edit::send(in_repo, sending)
    }

    fn send_checkout_as_pull_request(
        &self,
        in_repo: &str,
        sending: &adapter_traits::CheckoutToSend,
    ) -> Result<adapter_traits::CheckoutSent, NotDelivered> {
        crate::checkout_send::send(in_repo, sending)
    }
}

/// How a command is run: [`run_in`] or [`read_in`].
type Runner = fn(&str, &str, &[&str]) -> Result<Output, std::io::Error>;

/// Ask the forge one thing about one pull request, named as the forge names
/// them — a branch, a number or an address, all of which `pr view` takes.
///
/// **Every failure is `None`**, which is the rule
/// [`Landing`](adapter_traits::Landing) and [`already_open`] share: no tool, not signed in, no such pull request, a
/// network that would not answer. Nothing follows differently from any of them,
/// and where one is followed by a second call, that call is what says which it
/// was — putting the same fault on two lines is what this avoids.
///
/// **`--jq` and not a parse.** The forge does the reading, so bytes enter this
/// process as one line of text and `store` and `ipc` stay the only two places
/// that deserialise anything.
pub(crate) fn asked(in_dir: &str, named: &str, fields: &str, jq: &str) -> Option<String> {
    asked_by(read_in, in_dir, named, fields, jq)
}

/// [`asked`] through `run`: [`read_in`] for a poll, [`run_in`] for the look a
/// delivery takes before it acts.
fn asked_by(
    run: Runner,
    in_dir: &str,
    named: &str,
    fields: &str,
    jq: &str,
) -> Option<String> {
    let run = viewed(run, in_dir, named, fields, jq)?;
    run.status
        .success()
        .then(|| last_line(&run))
        .filter(|said| !said.is_empty())
}

/// The same ask, keeping every line it answers with.
///
/// **[`asked`]'s sibling rather than its replacement.** That one wants the
/// single value a scalar `--jq` prints and takes the last line, because `gh`
/// writes its notices to stdout above the answer. A `--jq` that prints a record
/// per line has no single line to take, so the notices come back with it — and
/// the one caller of this reads by a leading tag and drops every line it has no
/// tag for, which is that guard kept a different way.
///
/// **Every failure is `None`**, for [`asked`]'s reason exactly.
pub(crate) fn asked_lines(
    in_dir: &str,
    named: &str,
    fields: &str,
    jq: &str,
) -> Option<Vec<String>> {
    let run = viewed(read_in, in_dir, named, fields, jq)?;
    if !run.status.success() {
        return None;
    }
    // **Not trimmed.** `lines` already drops the separator and a trailing `\r`,
    // and what is left at the end of a record is a field — a comment ending in
    // a space is a comment ending in a space, and this is the one reader whose
    // payload was typed by somebody who is not here to be asked about it.
    let lines: Vec<String> = String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    (!lines.is_empty()).then_some(lines)
}

/// One `pr view`, built in one place so the two readers above cannot come to
/// spell the same command two ways.
fn viewed(
    run_in: Runner,
    in_dir: &str,
    named: &str,
    fields: &str,
    jq: &str,
) -> Option<Output> {
    run_in(
        in_dir,
        FORGE,
        &["pr", "view", named, "--json", fields, "--jq", jq],
    )
    .ok()
}

/// The pull request already open for this branch, if there is one.
///
/// Every failure is `None` — see [`asked`], which is where that rule lives now
/// that two callers keep it.
fn already_open(worktree: &Worktree) -> Option<String> {
    // A delivery's own look before it pushes: an act, so not at utility.
    asked_by(run_in, worktree.path(), worktree.branch(), "url", ".url")
}

/// The remote to push to: `origin` where there is one, otherwise the only other
/// name. **`None` is a repository with no remote**, which is ordinary.
///
/// Where several are configured and none is `origin`, nothing here can say
/// which one a person meant, so nothing is pushed.
pub(crate) fn a_remote(repo: &Repository) -> Option<String> {
    let remotes = repo.remotes().ok()?;
    let named: Vec<String> = remotes.iter().flatten().map(str::to_string).collect();
    match named.iter().any(|name| name == "origin") {
        true => Some(String::from("origin")),
        false => named.first().cloned(),
    }
}

/// The ordinary push. **The one Fleet takes**: a merge rewrites nothing, so
/// the remote reads it as a fast-forward and git refuses only a branch that
/// holds commits this one has not got — which a force would destroy.
pub(crate) fn pushed(worktree: &Worktree) -> Result<Pushed, NotDelivered> {
    let Some(remote) = a_remote(&open(worktree)?) else {
        return Ok(Pushed::NoRemote);
    };
    let branch = worktree.branch().to_string();
    let run = git(worktree, &["push", "--set-upstream", &remote, &branch])?;
    match run.status.success() {
        true => Ok(Pushed::ToTheRemote { remote, branch }),
        false => Err(NotDelivered::of("the push", said(&run))),
    }
}

/// The paths git is holding as unresolved, one per line.
pub(crate) fn unmerged_files(worktree: &Worktree) -> Vec<String> {
    let Ok(run) = git(worktree, &["diff", "--name-only", "--diff-filter=U"]) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn open(worktree: &Worktree) -> Result<Repository, NotDelivered> {
    Repository::open(worktree.path()).map_err(|cause| {
        NotDelivered::of("opening the Job's worktree", cause.message().to_string())
    })
}

fn head_of(repo: &Repository, worktree: &Worktree) -> Result<git2::Oid, NotDelivered> {
    repo.head()
        .ok()
        .and_then(|head| head.target())
        .ok_or_else(|| {
            NotDelivered::of(
                "reading the Job's branch",
                format!("`{}` points at no commit", worktree.branch()),
            )
        })
}

/// What the base branch tracks, and where that tracking branch points.
///
/// `None` at every step that could be absent — no such local branch, no
/// upstream configured, an upstream that points at nothing, a name git will not
/// spell. **None of them is an error**: each one means there is no second
/// reading of the base, which is a fact about the repository rather than a
/// failure of the call.
fn the_upstream_of(repo: &Repository, name: &str) -> Option<(String, git2::Oid)> {
    let upstream = repo
        .find_branch(name, BranchType::Local)
        .ok()?
        .upstream()
        .ok()?;
    let tip = upstream.get().target()?;
    Some((upstream.name().ok()??.to_string(), tip))
}

/// The commit a catch-up measures against: the remote's copy of `name` where
/// this machine has fetched one, else the local branch.
fn the_base_tip(repo: &Repository, name: &str) -> Result<git2::Oid, NotDelivered> {
    let read = fetching_the_base::base_ref(repo, name);
    repo.revparse_single(&read)
        .and_then(|found| found.peel_to_commit())
        .map(|commit| commit.id())
        .map_err(|_| {
            NotDelivered::of(
                "reading the base branch",
                format!("`{read}` points at no commit"),
            )
        })
}

fn tip_of(repo: &Repository, name: &str) -> Result<git2::Oid, NotDelivered> {
    repo.find_branch(name, BranchType::Local)
        .ok()
        .and_then(|found| found.get().target())
        .ok_or_else(|| {
            NotDelivered::of(
                "reading the base branch",
                format!("`{name}` points at no commit"),
            )
        })
}

/// Run `git` in the Job's worktree. **No shell**, like every other command
/// Armada runs, so nothing in an argument can be read as syntax.
pub(crate) fn git(worktree: &Worktree, args: &[&str]) -> Result<Output, NotDelivered> {
    run(worktree, "git", args).map_err(|why| NotDelivered::of("running git", why.to_string()))
}

fn run(worktree: &Worktree, program: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    run_in(worktree.path(), program, args)
}

/// The same, in a directory that is not a Job's worktree. **Every caller is in
/// `crate::landing`**, whose questions are asked long after the worktree the
/// work was written in has been reclaimed.
pub(crate) fn run_in(dir: &str, program: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    asked_at(Command::new(program), dir, args)
}

/// [`run_in`] for a question Fleet asks on its own account, at utility
/// priority: `crate::background` says why. **Every read goes here and every
/// act through `run_in`**, so a merge the owner pressed is not queued behind
/// the polling.
pub(crate) fn read_in(dir: &str, program: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    asked_at(crate::background::command(program), dir, args)
}

fn asked_at(mut command: Command, dir: &str, args: &[&str]) -> Result<Output, std::io::Error> {
    command
        .args(args)
        .current_dir(dir)
        // A tool that stops to ask has nobody to ask: Fleet is a daemon and
        // there is no terminal on the other end of this. Refusing is the
        // answer, and it comes back as the sentence the tool printed.
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
}

/// What a failed command said, both streams, for a person to read.
pub(crate) fn said(run: &Output) -> String {
    let err = String::from_utf8_lossy(&run.stderr);
    let out = String::from_utf8_lossy(&run.stdout);
    let joined = format!("{}\n{}", err.trim(), out.trim());
    joined.trim().to_string()
}

/// The last non-blank line of stdout. What a tool that prints an address prints
/// it as, after whatever progress it printed first.
pub(crate) fn last_line(run: &Output) -> String {
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

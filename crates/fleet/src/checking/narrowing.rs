//! What a Check runs once the gate or a Drone has said what changed: narrowed
//! to what the change reaches, or whole. Split from `checking` for size.

use checks_runner::{Narrowed, Reach};
use core_model::ResolvedCheck;
use verification::Observed;

use super::Planned;
use crate::gated::Gated;

/// What a Check runs when a Drone asked about its own change.
///
/// **Pure, and settled beside the skip decision** for the same reason: it reads
/// the Check's frozen `narrow` against paths already in hand, so nothing is
/// spawned and nothing is ordered by it.
///
/// **Three answers and each is a different sentence.** The Check runs whole,
/// which is what a Check with no `narrow` does; it runs a narrower command; or
/// it is not run at all, because nothing the change touched feeds it. The third is a skip and not a pass — a Drone told
/// `test` passed on a change that touched no crate would have been told
/// something false about its own work.
pub(crate) fn narrowed(
    check: &ResolvedCheck,
    name: &str,
    run: &str,
    touched: &[String],
    narrow: bool,
) -> Planned {
    let whole = Planned::Command {
        name: name.to_string(),
        run: run.to_string(),
        narrowed_to: None,
    };
    if !narrow {
        return whole;
    }
    match checks_runner::narrowed(check.narrowing(), touched) {
        // **The Check's own declaration is answered first, and the runner's
        // only where it said nothing.** A repository that wrote a `narrow` for
        // this Check has said how it wants it narrowed; a shipped description
        // of the runner is what answers where it has not. The same precedence
        // `docs/concepts/runner-adapter.md` states for a repository's own
        // description against a shipped one, one tier down.
        Narrowed::Whole => match by_its_runner(check, touched) {
            Some(command) => Planned::Command {
                name: name.to_string(),
                run: run.to_string(),
                narrowed_to: Some(command),
            },
            None => whole,
        },
        Narrowed::To(command) => Planned::Command {
            name: name.to_string(),
            run: run.to_string(),
            narrowed_to: Some(command),
        },
        // The same sentence a `when` skip writes, and it is the true one: a
        // Check that narrows to a directory the change did not touch has
        // nothing to say about the change. `Observed::Skipped` carries what a
        // reader's first question is, so it carries what the narrowing looked
        // at rather than what the Check covers.
        Narrowed::Nothing => Planned::Already(Observed::Skipped {
            covers: check
                .narrowing()
                .and_then(core_model::Narrowing::under)
                .unwrap_or_default()
                .to_string(),
        }),
    }
}

/// What a Check runs at the step gate: the merge line's reading, over what the
/// step's own change reaches — `checks_runner::narrowed_over`.
///
/// **Stricter than [`narrowed`], because this one rules.** Only `under`
/// narrows, any covered path it cannot name runs the Check whole, a verbatim
/// `narrow` never narrows, and no runner's description is asked: the merge line
/// asks none either. A change reaching only what the Check excludes is a skip,
/// as [`narrowed`]'s is.
pub(crate) fn at_the_gate(check: &ResolvedCheck, name: &str, run: &str, reach: &Reach) -> Planned {
    let command = |narrowed_to| Planned::Command {
        name: name.to_string(),
        run: run.to_string(),
        narrowed_to,
    };
    let Reach::Paths(reached) = reach else {
        return command(None);
    };
    match checks_runner::narrowed_over(check.narrowing(), reached, |path| {
        check.covers(std::slice::from_ref(path))
    }) {
        Narrowed::Whole => command(None),
        Narrowed::To(narrowed_to) => command(Some(narrowed_to)),
        Narrowed::Nothing => Planned::Already(Observed::Skipped {
            covers: check
                .narrowing()
                .and_then(core_model::Narrowing::under)
                .unwrap_or_default()
                .to_string(),
        }),
    }
}

/// Whether the step gate has a Check it could narrow over this change, which
/// is the only reason to ask `cargo tree` what the change reaches.
pub(crate) fn narrows_at_the_gate(
    checks: &[ResolvedCheck],
    touched: &[String],
    gated: Option<&Gated>,
) -> bool {
    checks.iter().any(|check| {
        check
            .narrowing()
            .and_then(core_model::Narrowing::under)
            .is_some()
            && check.covers(&crate::gated::local(gated, check, touched))
    })
}

/// The command this Check's runner narrows it to for these paths, where it has
/// one.
///
/// **A workspace Check with no runner `dir` gets `.`**, and its `touched` are
/// the manifest-relative paths `crate::gated::local` made. A root Check with
/// none still has nothing to put in `{dir}`.
///
/// **`None` is every reason a runner cannot answer**, and they are all the same
/// answer to the caller: the Check runs whole. It names no runner; no shipped
/// description answers to that name; the description declares no `run_changed`;
/// or the paths cannot be spelled as arguments.
///
/// A learned or repository-local description is not read here yet — `shipped`
/// is the only source, and the rest of the lifecycle in
/// `docs/concepts/runner-adapter.md` is unbuilt.
pub(crate) fn by_its_runner(check: &ResolvedCheck, touched: &[String]) -> Option<String> {
    let runner = check.runner()?;
    let described = config::shipped(runner.name())?;
    let Some(dir) = runner.dir() else {
        // A workspace Check runs in its own manifest's directory, so an omitted
        // `dir` is that directory, and its paths already arrive relative to it.
        let here = (!check.manifest_dir().is_empty()).then_some(".");
        return checks_runner::run_changed(described.run_changed()?, here, touched);
    };
    // The template runs from `dir`, so `{files}` must be dir-relative. A covered
    // path outside it cannot be named, and dropping it would narrow to a subset
    // nobody was told about, so the Check runs whole.
    let (inside, outside) = checks_runner::relative_to_dir(dir, touched);
    if outside
        .iter()
        .any(|path| check.covers(std::slice::from_ref(path)))
    {
        return None;
    }
    checks_runner::run_changed(described.run_changed()?, Some(dir), &inside)
}

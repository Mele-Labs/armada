use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::*;

fn part(name: &str, state: ReadingState) -> Part {
    Part::new(name.to_string(), state)
}

fn reading(state: ReadingState, finished: bool, parts: Vec<Part>) -> Reading {
    Reading { state, finished, says: "a sentence".to_string(), link: None, parts }
}

fn aside(part: &str) -> SetAside {
    SetAside { part: part.to_string(), sentence: "not ours".to_string() }
}

fn watch() -> Watch {
    Watch::new("QA deploy".to_string(), vec!["scripts/deploy-state".to_string()], 60, 7200)
}

#[test]
fn the_words_round_trip_and_an_unknown_one_is_none() {
    for state in ReadingState::ALL {
        assert_eq!(ReadingState::from_wire(state.as_wire()), Some(*state));
    }
    assert_eq!(ReadingState::from_wire("success"), None);
    for state in WatchState::ALL {
        assert_eq!(WatchState::from_wire(state.as_wire()), Some(*state));
    }
    assert_eq!(WatchState::from_wire("not_ours"), Some(WatchState::NotOurs));
}

/// Spike 36: a run reads `failed` overall while the parts that deploy the change still run.
#[test]
fn parts_decide_over_the_lines_own_state() {
    use ReadingState::*;
    let deploying = reading(Failed, false, vec![part("build", Passed), part("deploy-api", Running)]);
    assert_eq!(counted_state(&deploying, &[]), Running);

    let one_failed = reading(Running, false, vec![part("build", Failed), part("deploy-api", Running)]);
    assert_eq!(counted_state(&one_failed, &[]), Failed);

    let waiting = reading(Waiting, false, vec![part("build", Waiting), part("deploy-api", Passed)]);
    assert_eq!(counted_state(&waiting, &[]), Running, "a waiting part is running");

    let done = reading(Failed, true, vec![part("build", Passed), part("deploy-api", Passed)]);
    assert_eq!(counted_state(&done, &[]), Passed);
}

#[test]
fn without_parts_the_line_stands() {
    for state in ReadingState::ALL {
        assert_eq!(counted_state(&reading(*state, false, Vec::new()), &[aside("x")]), *state);
    }
}

#[test]
fn a_part_set_aside_no_longer_counts() {
    use ReadingState::*;
    let other = reading(Failed, true, vec![part("unit-tests-other", Failed), part("deploy-api", Passed)]);
    assert_eq!(counted_state(&other, &[]), Failed);
    assert_eq!(counted_state(&other, &[aside("unit-tests-other")]), Passed);

    let all_aside = reading(Failed, true, vec![part("flaky", Failed)]);
    assert_eq!(counted_state(&all_aside, &[aside("flaky")]), Passed, "nothing that counts failed");
}

/// The QA deploy that read green: `build` passed, `provision` failed and was set aside.
#[test]
fn a_set_aside_part_that_failed_ends_not_ours_and_never_passed() {
    use ReadingState::*;
    let deploy = reading(Failed, true, vec![part("build", Passed), part("provision", Failed)]);
    let set = [aside("provision")];
    assert_eq!(watch_state_of(&deploy, &set), WatchState::NotOurs);
    assert!(!ends_passed(&deploy, &set));
    assert!(ends_on(&deploy, &set));
    assert_eq!(set_aside_failing(&deploy, &set), vec!["provision"]);

    let all_aside = reading(Failed, true, vec![part("provision", Failed)]);
    assert_eq!(watch_state_of(&all_aside, &set), WatchState::NotOurs, "an empty set of counted parts is no pass");

    let still_going = reading(Failed, false, vec![part("build", Passed), part("provision", Failed)]);
    assert_eq!(watch_state_of(&still_going, &set), WatchState::NotOurs, "never green mid-run either");
    assert!(!ends_on(&still_going, &set));
}

#[test]
fn a_set_aside_part_that_passes_on_a_retry_blocks_nothing() {
    use ReadingState::*;
    let mut one = watch();
    one.set_aside = vec![aside("provision")];
    one.reported = vec!["provision".to_string()];
    assert_eq!(one.read(reading(Failed, false, vec![part("build", Passed), part("provision", Failed)])), WatchState::NotOurs);
    assert!(one.live);
    assert_eq!(one.read(reading(Passed, true, vec![part("build", Passed), part("provision", Passed)])), WatchState::Passed);
    assert!(!one.live);
}

#[test]
fn a_counted_failure_still_fails_beside_a_set_aside_one() {
    use ReadingState::*;
    let both = reading(Failed, true, vec![part("build", Failed), part("provision", Failed)]);
    assert_eq!(watch_state_of(&both, &[aside("provision")]), WatchState::Failed);
    assert!(!ends_on(&both, &[aside("provision")]), "a failure waits for the agent's verdict");
}

#[test]
fn a_watch_ends_passed_only_on_passed_and_finished() {
    use ReadingState::*;
    let mut unfinished = watch();
    assert_eq!(unfinished.read(reading(Passed, false, Vec::new())), WatchState::Passed);
    assert!(unfinished.live, "passed and not finished keeps watching");

    let mut finished = watch();
    finished.read(reading(Passed, true, Vec::new()));
    assert!(!finished.live);
    assert_eq!(finished.state, WatchState::Passed);

    let mut failed_at_end = watch();
    failed_at_end.read(reading(Failed, true, Vec::new()));
    assert!(failed_at_end.live, "a failure waits for the agent's verdict");
}

#[test]
fn not_ours_sets_parts_aside_and_counts_again() {
    use ReadingState::*;
    let mut one = watch();
    one.read(reading(Failed, true, vec![part("unit-tests-other", Failed), part("deploy-api", Passed)]));
    assert_eq!(one.state, WatchState::Failed);

    let verdict = Verdict::NotOurs { sentence: "another service".to_string(), parts: vec!["unit-tests-other".to_string()] };
    assert_eq!(one.weigh(verdict), Ok(WatchState::NotOurs));
    assert!(!one.live, "the last reading was finished, so it ends, and not passed");
    assert_eq!(one.set_aside, vec![SetAside { part: "unit-tests-other".to_string(), sentence: "another service".to_string() }]);
    let last = one.last.as_ref().map(|last| last.parts.iter().map(|part| part.state).collect::<Vec<_>>());
    assert_eq!(last, Some(vec![Failed, Passed]), "the reading keeps its real state");
}

#[test]
fn a_weighing_is_held_to_what_was_reported() {
    use ReadingState::*;
    let mut one = watch();
    one.read(reading(Failed, false, vec![part("build", Failed)]));
    let named = |parts: &[&str], sentence: &str| Verdict::NotOurs {
        sentence: sentence.to_string(),
        parts: parts.iter().map(|part| part.to_string()).collect(),
    };
    assert_eq!(one.weigh(named(&["deploy-web"], "why")), Err(NotWeighed::Unreported("deploy-web".to_string())));
    assert_eq!(one.weigh(named(&[], "why")), Err(NotWeighed::NoParts));
    assert_eq!(one.weigh(named(&["build"], "  ")), Err(NotWeighed::NoSentence));
    assert!(one.set_aside.is_empty(), "a refused weighing sets nothing aside");
}

fn ours(sentence: &str) -> Verdict {
    Verdict::Ours { sentence: sentence.to_string() }
}

#[test]
fn ours_ends_a_failed_watch_and_nothing_else() {
    use ReadingState::*;
    let mut running = watch();
    running.read(reading(Running, false, Vec::new()));
    assert_eq!(running.weigh(ours("Mine.")), Err(NotWeighed::NotFailed));

    let mut failed = watch();
    failed.read(reading(Failed, false, Vec::new()));
    assert_eq!(failed.weigh(ours(" ")), Err(NotWeighed::NoSentence), "ours says why, as not ours does");
    assert_eq!(failed.weigh(ours("The migration I added.")), Ok(WatchState::Failed));
    assert!(!failed.live);
    assert_eq!(failed.weigh(ours("Mine.")), Err(NotWeighed::Ended));
    assert_eq!(failed.read(reading(Passed, true, Vec::new())), WatchState::Failed, "an ended watch takes no reading");
}

#[test]
fn an_end_is_kept() {
    let mut one = watch();
    one.end(WatchState::Stopped);
    one.end(WatchState::GaveUp);
    assert_eq!(one.state, WatchState::Stopped);
    assert!(!one.live);
}

#[test]
fn a_span_is_a_count_and_a_unit() {
    assert_eq!(span_seconds("30s"), Some(30));
    assert_eq!(span_seconds("2m"), Some(120));
    assert_eq!(span_seconds("2h"), Some(7200));
    for refused in ["", "s", "0s", "60", "1.5h", "-1m", "2d", "1 h", "99999999999s"] {
        assert_eq!(span_seconds(refused), None, "{refused}");
    }
}

#[test]
fn a_value_fills_a_whole_item() {
    let command: Vec<String> = ["scripts/deploy-state", "{sha}", "--env={environment}", "{environment}"]
        .iter()
        .map(|item| item.to_string())
        .collect();
    let mut values = BTreeMap::new();
    values.insert("sha".to_string(), "abc; rm -rf /".to_string());
    values.insert("environment".to_string(), "qa".to_string());
    assert_eq!(
        fill(&command, &values),
        Ok(vec![
            "scripts/deploy-state".to_string(),
            "abc; rm -rf /".to_string(),
            "--env={environment}".to_string(),
            "qa".to_string(),
        ])
    );
}

#[test]
fn every_value_is_named_and_given() {
    let command = vec!["run".to_string(), "{sha}".to_string()];
    let given = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
    };
    assert_eq!(fill(&command, &given(&[])), Err(NotFilled::Missing("sha".to_string())));
    assert_eq!(fill(&command, &given(&[("sha", "a"), ("env", "qa")])), Err(NotFilled::Unknown("env".to_string())));
    assert_eq!(fill(&command, &given(&[("sha", " ")])), Err(NotFilled::Unusable("sha".to_string())));
    assert_eq!(fill(&command, &given(&[("sha", "a\0b")])), Err(NotFilled::Unusable("sha".to_string())));
}

//! `pr_mode:`, how a repository's pull requests are offered where nothing more
//! specific says.

use core_model::PrMode;

use crate::error::Fault;
use crate::manifest::Manifest;
use crate::tests::{fault_at, named, refusals};

fn parse(text: &str) -> Result<Manifest, crate::LoadError> {
    Manifest::parse(&named("armada.yml"), text)
}

/// **Absent defers**, which `ready` written does not: a repository that says
/// `ready` beats a machine that says `draft`.
#[test]
fn a_file_that_says_nothing_defers_and_ready_written_is_an_answer() {
    assert_eq!(
        parse("version: 1\nid: a\n").expect("optional").pr_mode(),
        None
    );
    let ready = parse("version: 1\nid: a\npr_mode: ready\n").expect("a value the key has");
    assert_eq!(ready.pr_mode(), Some(PrMode::Ready));
    let draft = parse("version: 1\nid: a\npr_mode: draft\n").expect("a value the key has");
    assert_eq!(draft.pr_mode(), Some(PrMode::Draft));
}

#[test]
fn a_word_the_key_has_no_value_for_is_refused_and_names_both() {
    let refused = refusals(parse("version: 1\nid: a\npr_mode: published\n"));
    let Fault::NotInTheSchema { legal, .. } = fault_at(&refused, "pr_mode") else {
        panic!("`published` is not a way a pull request is offered");
    };
    assert_eq!(*legal, ["ready", "draft"]);
}

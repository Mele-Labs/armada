//! What the API Client works out with no drawing and no Fleet: the `{{name}}`s of an address, what an environment fills them
//! to, the saved requests a typed name finds, what each chip of the list holds, and who a run was by.

use ipc::{RequestBy, RequestPlace, RequestsNotRead};

use super::model::{self, Filter, Line, SentLine};
use super::test_fixtures::{collection, environment, plain, requests, run, saved, secret, standard};

fn sent(id: &str, by: RequestBy) -> SentLine {
    SentLine { id: id.into(), by, method: "GET".into(), url: format!("http://localhost/{id}"), status: Some(200), error: None }
}

// ---- the request line ---------------------------------------------------------------------------

#[test]
fn each_name_in_braces_is_found_with_where_it_stands_and_room_inside_is_allowed() {
    let found = model::names("{{base}}/users/{{ id }}?q={{a.b-c_1}}");
    assert_eq!(found, vec![(0..8, "base".to_string()), (15..23, "id".to_string()), (26..37, "a.b-c_1".to_string())]);
    assert_eq!(model::names("{{{base}}}"), vec![(1..9, "base".to_string())], "a brace before a name is not part of it");
}

#[test]
fn a_braced_text_that_is_not_a_name_is_not_found() {
    for not_a_name in ["{{}}", "{{ }}", "{{a b}}", "{base}", "{{base}", "{{ba/se}}", "base}}", "{{base", "plain text"] {
        assert!(model::names(not_a_name).is_empty(), "{not_a_name:?} holds no name");
    }
}

#[test]
fn typing_part_of_a_name_finds_the_saved_requests_whose_name_holds_it_in_the_list_order() {
    let read = standard();
    assert_eq!(model::found(&read, "heal", 6), vec![(0, 0)]);
    assert_eq!(model::found(&read, "  USER ", 6), vec![(0, 1), (0, 2)], "case does not matter, nor the room round it");
    assert!(model::found(&read, "orders", 6).is_empty(), "a name no request has finds nothing");
    assert_eq!(model::found(&read, "user", 1), vec![(0, 1)], "no more than the limit");
}

#[test]
fn an_address_is_not_taken_for_a_name() {
    let read = standard();
    for address in ["", "   ", "http://heal", "/health", "{{base}}", "heal/th", "{{"] {
        assert!(model::found(&read, address, 6).is_empty(), "{address:?} reads as an address, so it finds no request");
    }
}

// ---- the list's chips ---------------------------------------------------------------------------

fn lines_of(filter: Filter, sends: &[SentLine]) -> Vec<Line> {
    model::lines(Some(&standard()), sends, filter, None)
}

fn thread() -> Vec<SentLine> {
    vec![sent("first", RequestBy::Agent), sent("second", RequestBy::Owner), sent("third", RequestBy::Agent)]
}

#[test]
fn this_job_holds_the_saved_requests_and_every_send_of_the_job_newest_first() {
    assert_eq!(
        lines_of(Filter::ThisJob, &thread()),
        vec![
            Line::Collection(0),
            Line::Saved(0, 0),
            Line::Saved(0, 1),
            Line::Saved(0, 2),
            Line::Heading("Sent in this Job".into()),
            Line::Sent(2),
            Line::Sent(1),
            Line::Sent(0),
        ]
    );
}

#[test]
fn saved_holds_the_saved_requests_and_none_of_the_jobs_sends() {
    assert_eq!(lines_of(Filter::Saved, &thread()), vec![Line::Collection(0), Line::Saved(0, 0), Line::Saved(0, 1), Line::Saved(0, 2)]);
}

#[test]
fn agent_and_mine_hold_only_what_that_side_sent_and_no_saved_request() {
    assert_eq!(lines_of(Filter::Agent, &thread()), vec![Line::Heading("Sent in this Job".into()), Line::Sent(2), Line::Sent(0)]);
    assert_eq!(lines_of(Filter::Mine, &thread()), vec![Line::Heading("Sent in this Job".into()), Line::Sent(1)]);
}

#[test]
fn a_chip_with_nothing_to_show_draws_no_heading() {
    let agent_only = vec![sent("only", RequestBy::Agent)];
    assert!(lines_of(Filter::Mine, &agent_only).is_empty(), "nothing of his own sends, so no heading over nothing");
    assert_eq!(lines_of(Filter::Saved, &[]), vec![Line::Collection(0), Line::Saved(0, 0), Line::Saved(0, 1), Line::Saved(0, 2)]);
}

#[test]
fn the_agent_is_the_one_who_is_not_the_owner() {
    assert_eq!(sent("a", RequestBy::Owner).who(), "You");
    assert_eq!(sent("a", RequestBy::Agent).who(), "Agent");
}

// ---- the environment ----------------------------------------------------------------------------

fn staging() -> ipc::RequestEnvironmentRead {
    environment("staging", vec![plain("base", "https://staging.example.test"), secret("token", true), secret("old", false)], RequestPlace::Yours).item
}

#[test]
fn a_plain_variable_fills_its_name() {
    let filled = model::filled("{{base}}/orders?x={{ base }}", Some(&staging()));
    assert_eq!(filled.text, "https://staging.example.test/orders?x=https://staging.example.test");
    assert!(filled.missing.is_empty());
}

#[test]
fn a_secret_stays_its_name_since_bridge_never_holds_one() {
    let filled = model::filled("Bearer {{token}}", Some(&staging()));
    assert_eq!(filled.text, "Bearer {{token}}");
    assert!(filled.missing.is_empty(), "a secret the Keychain holds a value for is not missing, as in Classic");
}

#[test]
fn a_secret_nothing_was_kept_for_and_a_name_the_environment_lacks_stay_and_are_reported_once_in_order() {
    let filled = model::filled("{{tenant}}/{{old}}/{{base}}/{{tenant}}", Some(&staging()));
    assert_eq!(filled.text, "{{tenant}}/{{old}}/https://staging.example.test/{{tenant}}");
    assert_eq!(filled.missing, ["tenant", "old"]);
}

#[test]
fn with_no_environment_every_name_is_missing_and_the_text_is_as_typed() {
    let filled = model::filled("{{base}}/x/{{id}}", None);
    assert_eq!(filled.text, "{{base}}/x/{{id}}");
    assert_eq!(filled.missing, ["base", "id"]);
}

#[test]
fn a_servers_environment_fills_base_with_the_address_the_server_listens_on() {
    let read = standard();
    let server = read.environments.iter().find(|one| one.item.name == "server:api").expect("the Job's server is listed");
    assert_eq!(model::filled("{{base}}/health", Some(&server.item)).text, "http://localhost:4300/health");
}

// ---- saving -------------------------------------------------------------------------------------

#[test]
fn only_the_persons_own_file_not_reading_turns_saving_off() {
    let mut read = requests(vec![collection("c", "C", Vec::new(), vec![saved("a", "A", "GET", "/a")], RequestPlace::Yours)], Vec::new());
    assert!(model::yours_unread(Some(&read)).is_none());
    assert!(model::yours_unread(None).is_none(), "no read, nothing to refuse");
    read.not_read.push(RequestsNotRead { place: RequestPlace::Repository, why: "a table has no name".into() });
    read.not_read.push(RequestsNotRead { place: RequestPlace::Server, why: "no port".into() });
    assert!(model::yours_unread(Some(&read)).is_none(), "the repository's file and a server's environment do not lock Save");
    read.not_read.push(RequestsNotRead { place: RequestPlace::Yours, why: "expected `=` at line 3".into() });
    let unread = model::yours_unread(Some(&read)).expect("the person's own file is the one");
    assert_eq!(unread.place, RequestPlace::Yours);
    assert_eq!(model::unread_said(unread), "expected `=` at line 3. Save is off until it reads.");
}

// ---- runs ---------------------------------------------------------------------------------------

#[test]
fn a_run_is_by_you_the_agent_or_the_title_of_another_job() {
    let here = Some(("this-job", Some("Add the model picker")));
    let other = Some(("other-job", Some("Add rate limits")));
    let untitled = Some(("other-job", None));
    let blank = Some(("other-job", Some("  ")));
    assert_eq!(model::run_who(&run("a", RequestBy::Owner, here, Some(200)), "this-job"), "You");
    assert_eq!(model::run_who(&run("b", RequestBy::Agent, here, Some(200)), "this-job"), "Agent");
    assert_eq!(model::run_who(&run("c", RequestBy::Agent, other, Some(200)), "this-job"), "Add rate limits", "another Job's run is by that Job, whoever in it sent it");
    assert_eq!(model::run_who(&run("d", RequestBy::Owner, other, Some(200)), "this-job"), "Add rate limits");
    assert_eq!(model::run_who(&run("e", RequestBy::Owner, untitled, Some(200)), "this-job"), "Another Job");
    assert_eq!(model::run_who(&run("f", RequestBy::Owner, blank, Some(200)), "this-job"), "Another Job");
    assert_eq!(model::run_who(&run("g", RequestBy::Owner, None, Some(200)), "this-job"), "You", "a run with no Job named is by whoever sent it");
    assert_eq!(model::run_who(&run("h", RequestBy::Agent, None, Some(200)), "this-job"), "Agent");
}

#[test]
fn a_run_is_answered_failed_or_not_answered_by_its_status() {
    use model::Outcome;
    assert_eq!(model::outcome_of(Some(200)), Outcome::Answered);
    assert_eq!(model::outcome_of(Some(302)), Outcome::Answered);
    assert_eq!(model::outcome_of(Some(404)), Outcome::Failed);
    assert_eq!(model::outcome_of(Some(503)), Outcome::Failed);
    assert_eq!(model::outcome_of(None), Outcome::None);
    assert_eq!(model::status_said(Some(503), None), "503");
    assert_eq!(model::status_said(None, Some("connection refused")), "No answer: connection refused");
    assert_eq!(model::status_said(None, None), "No answer");
}

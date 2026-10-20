//! The issues a Session works on, kept as `issue` rows on the ledger: the ones its pull requests
//! close, as the forge names them on the `pr` row's `closes`. A scripted forge and tracker stand in
//! for `gh`.

use adapter_traits::PullRequestStanding;
use api::PullRequests;

use crate::tests::pull_requesting::{
    a_fleet, a_session_holding_pull_request_12, facts, forge_ran, manifest, passed, the_row,
};
use crate::tests::tmp::TempDir;

#[tokio::test]
async fn a_pull_requests_row_says_which_issues_it_closes_and_says_nothing_where_none() {
    let home = TempDir::new();
    let fleet = a_fleet(&home);
    let (id, _) = manifest(&fleet);
    a_session_holding_pull_request_12(&fleet, "s1").await;
    forge_ran(&fleet, passed());
    fleet.vcs().now_pull_request(Some(adapter_traits::PullRequestFacts {
        closes: vec![7, 31],
        ..facts(PullRequestStanding::Open)
    }));

    let read = fleet.get_pull_request(id.clone(), 12).await.expect("read");

    assert_eq!(read.closes, vec![7, 31]);
    let row = the_row(&fleet, "s1", "pr").await;
    assert_eq!(row.detail.get("closes").map(String::as_str), Some("7,31"));

    fleet.vcs().now_pull_request(Some(facts(PullRequestStanding::Open)));
    fleet.get_pull_request(id, 12).await.expect("read");

    assert_eq!(the_row(&fleet, "s1", "pr").await.detail.get("closes"), None);
}

//! A refresh of the preview starts from what the running build holds and what he picked, and adds the offers on top.
//! An offer arriving never drops a held branch; a Job that withdraws the branch does, until it offers it again.

use std::fs;
use std::sync::Arc;

use api::Sessions;
use ipc::{PreviewOffers, SessionId, WithdrawPreview};

use super::build_notices::{a_rig, Rig};
use super::preview_offering::{a_job_offering, a_previewable_repository, base, branch_with, checked, holding, offer};

fn file(rig: &Rig) -> String {
    fs::read_to_string(rig.support().join("preview-offered")).unwrap_or_default()
}

async fn read(rig: &Rig) -> PreviewOffers {
    rig.fleet.preview_offers_read().await
}

fn kept(read: &PreviewOffers) -> Vec<String> {
    read.refresh.as_ref().map(|refresh| refresh.keeps.iter().map(|one| one.branch.clone()).collect()).unwrap_or_default()
}

#[tokio::test]
async fn a_refresh_keeps_the_branches_the_running_build_holds_and_the_picks_and_adds_the_new_offer() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let one = branch_with(&rig, "armada/held-one", &base(&rig), "crates/x/held_one.rs", "1");
    let two = branch_with(&rig, "armada/held-two", &base(&rig), "crates/x/held_two.rs", "2");
    let picked = branch_with(&rig, "armada/picked", &base(&rig), "crates/x/picked.rs", "3");
    holding(&rig, &[("armada/held-one", &one), ("armada/held-two", &two)]);
    fs::write(rig.support().join("preview-picked"), "armada/held-one\narmada/held-two\narmada/picked\n").unwrap();

    let (session, tip) = a_job_offering(&rig, "armada/new", "crates/x/new.rs", "4").await;
    offer(&rig, &session, None, Some("New")).await.expect("offered");
    checked(&rig).await;

    assert_eq!(
        file(&rig),
        format!("armada/held-one\t{one}\t\t\narmada/held-two\t{two}\t\t\narmada/picked\t{picked}\t\t\narmada/new\t{tip}\t{session}\tNew\n"),
        "the held branches first at their held tips, then the pick, then the offer"
    );
    let read = read(&rig).await;
    let refresh = read.refresh.as_ref().expect("a refresh row");
    assert_eq!(refresh.branches, ["armada/new", "armada/picked"], "what Refresh brings in");
    assert_eq!(kept(&read), ["armada/held-one", "armada/held-two"], "what it keeps");
}

#[tokio::test]
async fn a_held_branch_its_job_withdraws_leaves_the_next_refresh_and_comes_back_when_offered_again() {
    let rig = a_rig();
    a_previewable_repository(&rig);
    let (session, tip) = a_job_offering(&rig, "armada/one", "crates/x/one.rs", "1").await;
    offer(&rig, &session, None, Some("One")).await.expect("offered");
    checked(&rig).await;
    holding(&rig, &[("armada/one", &tip)]);
    let (other, other_tip) = a_job_offering(&rig, "armada/two", "crates/x/two.rs", "2").await;
    offer(&rig, &other, None, Some("Two")).await.expect("offered");
    checked(&rig).await;
    assert_eq!(file(&rig), format!("armada/one\t{tip}\t{session}\tOne\narmada/two\t{other_tip}\t{other}\tTwo\n"));

    let withdraw = WithdrawPreview { session_id: Some(SessionId::carried(session.as_str())), branch: None };
    Arc::clone(&rig.fleet).withdraw_preview(None, withdraw).await.expect("withdrawn");
    checked(&rig).await;
    assert_eq!(file(&rig), format!("armada/two\t{other_tip}\t{other}\tTwo\n"), "the build holds it, but its Job took it back");
    assert!(kept(&read(&rig).await).is_empty());

    offer(&rig, &session, None, Some("One again")).await.expect("offered again");
    checked(&rig).await;
    assert_eq!(file(&rig), format!("armada/two\t{other_tip}\t{other}\tTwo\narmada/one\t{tip}\t{session}\tOne again\n"), "offered last, so merged last");
}

//! The claim, written before the code: **a page may link the repository's own stylesheets by their path from its root.
//! Fleet keeps them with the page as it keeps a `mock`'s, under `page` in the document's `kept_styles`, and serves the
//! newest revision with their rules where the first such link stood and every other link gone. A sheet it cannot keep
//! leaves the page kept and tells the Job why. The policy lets a font load from a `data:` address and nothing else.**
//! `crates/ipc/operations/get_artifact_page.toml`.

use std::collections::BTreeMap;
use std::path::Path;

use api::{HostedSessions, Sessions};
use ipc::SessionId;

use super::page_actions::{a_job, page_holding};
use super::session_host::{eventually, rig_placing, Rig};

const PAGE: &str = "<!doctype html><html><head><title>Card</title>\
    <link rel=\"stylesheet\" href=\"armada-kit.css\"><link rel=\"stylesheet\" href=\"src/app.css\">\
    <link rel=\"stylesheet\" href=\"src/more.css\"><link rel=\"stylesheet\" href=\"https://example.com/far.css\">\
    </head><body><h1 class=\"proto-card\">Card</h1></body></html>";

fn write(rig: &Rig, files: &[(&str, &str)]) {
    for (path, text) in files {
        let file = Path::new(&rig.root).join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
}

/// The document's `kept_styles`, as its row keeps it.
async fn kept_of(rig: &Rig, id: &SessionId, path: &str) -> Option<BTreeMap<String, String>> {
    let home = rig.fleet.page_home(id.as_str(), path);
    let listed = rig.fleet.list_sessions(None, None, None).await.unwrap().sessions;
    let document = listed.iter().find(|one| &one.id == id)?.attachments.iter().find(|a| a.target == home)?.clone();
    document.detail.get("kept_styles").map(|kept| ipc::decode("kept styles", kept.as_bytes()).unwrap())
}

#[tokio::test]
async fn a_page_linking_repository_stylesheets_keeps_them_and_is_served_with_their_rules_in_place_of_its_links() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    write(&rig, &[("src/app.css", ".proto-card { color: #e11d48 }"), ("src/more.css", ".proto-more { margin: 3px }")]);
    let path = page_holding(&rig, &id, at, PAGE).await;

    let kept = kept_of(&rig, &id, &path).await.expect("the page keeps a style set");
    assert_eq!(kept.keys().collect::<Vec<_>>(), ["page"]);
    let set = rig.fleet.get_kept_styles(kept["page"].clone()).await.unwrap();
    assert_eq!(set.sheets.iter().map(|one| one.path.as_str()).collect::<Vec<_>>(), ["src/app.css", "src/more.css"]);

    for served in [
        rig.fleet.get_artifact_page(id.clone(), path.clone(), None).await.unwrap(),
        rig.fleet.get_hosted_artifact_page(id.clone(), path.clone(), None).await.unwrap(),
    ] {
        let html = &served.html;
        assert!(html.contains(".proto-card { color: #e11d48 }") && html.contains(".proto-more"), "the repository's rules are inline: {html}");
        assert!(!html.contains("<link") && !html.contains("src/app.css") && !html.contains("far.css"), "no link is left to fetch");
        assert!(html.find("li::marker").unwrap() < html.find(".proto-card { color").unwrap(), "after the kit, so the repository's rules win");
        assert!(served.policy.contains("font-src data:"), "{}", served.policy);
    }
}

#[tokio::test]
async fn a_page_linking_a_sheet_that_is_not_there_is_kept_and_the_job_is_told_why() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let page = PAGE.replace("src/app.css", "src/gone.css");
    let path = page_holding(&rig, &id, at, &page).await;

    assert!(kept_of(&rig, &id, &path).await.is_none(), "nothing was kept for the page");
    let served = rig.fleet.get_artifact_page(id.clone(), path.clone(), None).await.expect("the page is kept and served");
    assert!(served.html.contains("proto-card") && !served.html.contains("<link"), "{}", served.html);
    eventually(|| async {
        rig.rows(&id).await.iter().any(|row| {
            matches!(row, ipc::SessionRow::Message { notice: Some(word), text, .. }
                if word == "document_sketch" && text.contains("the page's stylesheets will not draw") && text.contains("src/gone.css"))
        })
    })
    .await;
}

#[tokio::test]
async fn a_page_with_no_repository_link_keeps_no_style_set() {
    let rig = rig_placing(None);
    let (id, at) = a_job(&rig).await;
    let path = page_holding(&rig, &id, at, "<html><head><link rel=\"stylesheet\" href=\"armada-kit.css\"></head><body><p>Plain</p></body></html>").await;
    assert!(kept_of(&rig, &id, &path).await.is_none());
}

//! Requests' collections and environments as a Share piece: made from the person's own file, read
//! and added to a colleague's. **A secret is its name alone in the package**: the value stays in
//! the sharer's Keychain, and the colleague's environment holds the name with nothing set.

use api::Requests;
use ipc::{
    Landed, PackageText, PieceKind, PieceToCommit, RequestCollection, RequestEnvironmentWrite, SaveRequestCollection,
    SaveRequestEnvironment, VariableRead, VariableWrite,
};

use super::{adding, mac, refused_with, Mac, REMOTE};

const TOKEN: &str = "t0ken-0f-the-0wner";

impl Mac {
    async fn keeps_requests(&self) {
        let environment = RequestEnvironmentWrite {
            name: "staging".into(),
            variables: vec![
                VariableWrite::Plain { name: "base".into(), value: "https://staging.example.test".into() },
                VariableWrite::Secret { name: "token".into(), value: Some(TOKEN.into()) },
            ],
        };
        let save = SaveRequestEnvironment { repository: Some(self.root.clone()), session_id: None, environment };
        self.fleet.save_request_environment(None, save).await.expect("kept");
        let collection = RequestCollection { id: "users".into(), name: "Users".into(), headers: Vec::new(), requests: Vec::new() };
        let save = SaveRequestCollection { repository: Some(self.root.clone()), session_id: None, collection };
        self.fleet.save_request_collection(None, save).await.expect("kept");
    }
}

fn requests(collections: &[&str], environments: &[&str]) -> PieceToCommit {
    PieceToCommit::Requests {
        collections: collections.iter().map(|one| one.to_string()).collect(),
        environments: environments.iter().map(|one| one.to_string()).collect(),
    }
}

#[tokio::test]
async fn requests_are_shared_with_a_secrets_name_and_added_to_the_colleagues_own() {
    let mine = mac(REMOTE);
    mine.keeps_requests().await;
    let made = mine.fleet.package_made(mine.sharing(requests(&["users"], &["staging"]))).await.expect("made");
    assert_eq!(made.file_name, "requests.armada.toml");
    assert!(made.text.contains("name = \"token\"") && made.text.contains("secret = true"), "{}", made.text);
    assert!(!made.text.contains(TOKEN) && !made.text.contains(&mine.root), "{}", made.text);

    let theirs = mac(REMOTE);
    let read = theirs.fleet.package_read(PackageText { text: made.text.clone() }).await.expect("read");
    let piece = &read.pieces[0];
    assert_eq!((piece.kind, piece.name.as_str(), piece.exists), (PieceKind::Requests, "Users, staging", false));
    let added = theirs.fleet.package_added(adding(&made.text, false)).await.expect("added");
    assert_eq!(added.pieces[0].landed, Landed::Added);
    let listed = theirs.fleet.get_requests(None, Some(theirs.root.clone()), None).await.expect("read");
    assert_eq!(listed.collections[0].item.id, "users");
    assert_eq!(listed.environments[0].item.variables[1], VariableRead::Secret { name: "token".into(), set: false }, "the value stayed behind");

    let (status, code, _) = refused_with(theirs.fleet.package_added(adding(&made.text, false)).await.expect_err("held"));
    assert_eq!((status, code.as_str()), (422, "fleet.share_exists"));
    let again = theirs.fleet.package_added(adding(&made.text, true)).await.expect("replaced");
    assert_eq!(again.pieces[0].landed, Landed::Replaced);

    let (status, code, _) = refused_with(mine.fleet.package_made(mine.sharing(requests(&["nobody"], &[]))).await.expect_err("refused"));
    assert_eq!((status, code.as_str()), (422, "fleet.share_not_yours"));
    let leaked = made.text.replace("secret = true", &format!("secret = true\nvalue = \"{TOKEN}\""));
    let (_, code, said) = refused_with(theirs.fleet.package_read(PackageText { text: leaked }).await.expect_err("refused"));
    assert_eq!(code, "fleet.share_malformed");
    assert!(!said.contains(TOKEN), "{said}");
}

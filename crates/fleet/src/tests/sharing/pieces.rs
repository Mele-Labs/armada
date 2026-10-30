//! Share does not carry Workspaces, lifecycles, rules or autonomy defaults yet, and says so.

use ipc::{PieceScope, PieceToCommit};

use super::{mac, refused_with, REMOTE};

#[tokio::test]
async fn a_workspace_a_lifecycle_a_rule_and_autonomy_defaults_are_refused_with_a_sentence() {
    let mine = mac(REMOTE);
    let everywhere = PieceScope::Everywhere;
    for piece in [
        PieceToCommit::Lifecycle { id: "ticket".into(), scope: everywhere.clone() },
        PieceToCommit::LifecycleRule { id: "nudge".into(), scope: everywhere.clone() },
        PieceToCommit::Workspace { id: "my-store".into() },
        PieceToCommit::Autonomy { id: "mine".into(), scope: everywhere },
    ] {
        let (status, code, said) = refused_with(mine.fleet.package_made(mine.sharing(piece)).await.expect_err("refused"));
        assert_eq!((status, code.as_str()), (422, "fleet.share_refused"));
        assert_eq!(said, "Share does not carry Workspaces, lifecycles, rules or autonomy defaults yet.");
    }
}

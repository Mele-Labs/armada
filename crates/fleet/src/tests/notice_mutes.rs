//! `set_notice_mute` keeps a mute where the behind-base notice reads it, an hour mute ends an hour on,
//! and unmuting lifts it.

use api::Commands;
use ipc::{Notice, NoticeScope, SetNoticeMute};
use testkit::FakeWorkProduct;

use crate::tests::tmp::TempDir;

fn mute(scope: NoticeScope, muted: bool) -> SetNoticeMute {
    SetNoticeMute { notice: Notice::BehindBase, scope, muted }
}

#[tokio::test]
async fn a_mute_is_kept_for_the_notice_and_lifted_by_unmuting() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let word = Notice::BehindBase.word();
    let now = "2000-01-01T00:00:00.000Z";

    fleet.set_notice_mute(mute(NoticeScope::Repo { manifest_id: "m1".into() }, true)).await.unwrap();
    fleet.set_notice_mute(mute(NoticeScope::Job { session_id: "s1".into() }, true)).await.unwrap();
    let held = fleet.store().lock().await.live_mutes(word, now).unwrap();
    assert!(held.covers("other", "m1") && held.covers("s1", "other"));

    fleet.set_notice_mute(mute(NoticeScope::Repo { manifest_id: "m1".into() }, false)).await.unwrap();
    let held = fleet.store().lock().await.live_mutes(word, now).unwrap();
    assert!(!held.covers("other", "m1"));
}

#[tokio::test]
async fn an_hour_mute_ends_after_an_hour_and_a_nameless_one_is_refused() {
    let home = TempDir::new();
    let fleet = crate::tests::daemon::a_fleet(&home, FakeWorkProduct::changed(&[]));
    let word = Notice::BehindBase.word();
    fleet.set_notice_mute(mute(NoticeScope::Hour { session_id: "s1".into() }, true)).await.unwrap();
    let now = fleet.now();
    let store = fleet.store().lock().await;
    assert!(store.is_muted(word, "s1", "m", now.as_str()).unwrap(), "held now");
    assert!(!store.is_muted(word, "s1", "m", "2999-01-01T00:00:00.000Z").unwrap(), "ended by then");
    drop(store);

    let refused = fleet.set_notice_mute(mute(NoticeScope::Job { session_id: " ".into() }, true)).await;
    assert!(refused.is_err());
}

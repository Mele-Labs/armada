//! Kit kept the same on two linked Macs over the link: a kind switched off sends nothing, a kind switched on crosses
//! a new item both ways, an edit on one side crosses, an edit on both is held as a clash with both sides, keeping
//! either side settles it on both Macs (one of them asleep when it is kept), and a piece holding a credential stays
//! where it is and says why. Each Mac's harness home is held in memory; an edit "outside Armada" is a write into it
//! that the Mac's next Kit read notices.

use std::sync::Arc;

use api::SharedHome;
use ipc::{HomeItem, HomeKind, KitAgainst, KitKeep, KitLink, KitMark, MacLink, SetKitLinkKind, SettleKitClash};
use testkit::{FakeHarnessHome, HeldFolder};

use super::{paired, until, OneMac, Ticking};
use crate::home::HomeFittings;

/// One Mac with Kit's folders in memory, and its harness home to plant in and read back.
struct Kitted {
    mac: OneMac,
    harness: HeldFolder,
}

fn skill(name: &str) -> String {
    format!("harness/skills/{name}/SKILL.md")
}

impl Kitted {
    async fn new(name: &str, ids: &'static str, clock: &Arc<Ticking>) -> Kitted {
        let mac = OneMac::new(name, ids, clock).await;
        let harness = HeldFolder::new();
        mac.rig.fleet.sharing_home_with(HomeFittings {
            mods: Arc::new(HeldFolder::new()),
            kit: Arc::new(HeldFolder::new()),
            harness: Arc::new(FakeHarnessHome::over(harness.clone())),
        });
        Kitted { mac, harness }
    }

    /// A skill written as he would outside Armada, then noticed: the next Kit read sees it and tells the other Mac.
    async fn wrote(&self, name: &str, text: &str) {
        self.harness.clone().with(&skill(name), text);
        self.read().await;
    }

    fn text(&self, name: &str) -> Option<String> {
        self.harness.text(&skill(name))
    }

    async fn switched(&self, kind: HomeKind, on: bool) {
        self.mac.rig.fleet.set_kit_link_kind(SetKitLinkKind { kind, on }).await.expect("switched");
    }

    async fn read(&self) -> KitLink {
        self.mac.rig.fleet.get_kit_link().await.expect("read")
    }

    /// How skill `name` stands against Mac `other`, where a row names it.
    async fn against(&self, name: &str, other: &str) -> Option<KitAgainst> {
        let link = self.read().await;
        let row = link.rows.into_iter().find(|row| row.kind == HomeKind::Skills && row.name == name)?;
        row.against.into_iter().find(|one| one.mac == other)
    }

    async fn mark(&self, name: &str, other: &str) -> Option<KitMark> {
        self.against(name, other).await.map(|one| one.mark)
    }

    /// Whether `other`'s Kit has crossed here at least once.
    async fn heard(&self, other: &str) -> bool {
        self.read().await.macs.iter().any(|mac| mac.mac == other && mac.crossed_at.is_some())
    }
}

/// Both Macs, Kit's folders fitted, `skills` switched on each as asked, paired, each having heard the other.
async fn linked(home_on: bool, work_on: bool) -> (Kitted, Kitted) {
    let clock = Arc::new(Ticking::from_nine());
    let (home, work) = (Kitted::new("Home", "01HMAC", &clock).await, Kitted::new("Work", "01WMAC", &clock).await);
    home.switched(HomeKind::Skills, home_on).await;
    work.switched(HomeKind::Skills, work_on).await;
    (home, work)
}

async fn pair(home: &Kitted, work: &Kitted) {
    paired(&home.mac, &work.mac).await;
    let (home_id, work_id) = (home.mac.id.clone(), work.mac.id.clone());
    until("Home's Kit crosses to Work", || async { work.heard(&home_id).await }).await;
    until("Work's Kit crosses to Home", || async { home.heard(&work_id).await }).await;
}

const TONE: &str = "---\ndescription: house tone\n---\nShort sentences.\n";
const LINT: &str = "---\ndescription: lint first\n---\nRun the linter.\n";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_kind_switched_off_sends_nothing_and_takes_nothing() {
    let (home, work) = linked(false, true).await;
    home.wrote("tone", TONE).await;
    work.wrote("lint", LINT).await;
    pair(&home, &work).await;

    let offer = home.mac.rig.fleet.kit_offered().await.expect("Home's offer");
    assert!(offer.kinds.is_empty() && offer.pieces.is_empty(), "Home sends nothing: {offer:?}");
    // Work has heard Home and has skills on, and still has no tone; Home has skills off and took no lint.
    assert_eq!(work.text("tone"), None, "Home's skill did not cross");
    assert_eq!(home.text("lint"), None, "Work's skill did not cross to a Mac with skills off");
    assert_eq!(work.mark("lint", &home.mac.id).await, None, "nothing to stand against on a Mac that sends no skills");
    assert!(home.read().await.rows.is_empty(), "a kind off has no rows");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_kind_switched_on_crosses_a_new_item_both_ways() {
    let (home, work) = linked(true, true).await;
    home.wrote("tone", TONE).await;
    work.wrote("lint", LINT).await;
    pair(&home, &work).await;
    let (home_id, work_id) = (home.mac.id.clone(), work.mac.id.clone());

    until("tone reaches Work", || async { work.text("tone").as_deref() == Some(TONE) }).await;
    until("lint reaches Home", || async { home.text("lint").as_deref() == Some(LINT) }).await;
    until("both read the same on Work", || async {
        work.mark("tone", &home_id).await == Some(KitMark::Same) && work.mark("lint", &home_id).await == Some(KitMark::Same)
    })
    .await;
    until("and on Home", || async {
        home.mark("tone", &work_id).await == Some(KitMark::Same) && home.mark("lint", &work_id).await == Some(KitMark::Same)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_edit_on_one_side_since_they_last_agreed_crosses_by_itself() {
    let (home, work) = linked(true, true).await;
    home.wrote("tone", TONE).await;
    pair(&home, &work).await;
    let home_id = home.mac.id.clone();
    until("tone reaches Work", || async { work.mark("tone", &home_id).await == Some(KitMark::Same) }).await;

    let edited = "---\ndescription: house tone\n---\nShorter sentences.\n";
    home.wrote("tone", edited).await;
    until("Home's edit reaches Work", || async { work.text("tone").as_deref() == Some(edited) }).await;

    let back = "---\ndescription: house tone\n---\nShorter sentences, and no dashes.\n";
    work.wrote("tone", back).await;
    until("Work's edit reaches Home", || async { home.text("tone").as_deref() == Some(back) }).await;
    until("and both read the same", || async { work.mark("tone", &home_id).await == Some(KitMark::Same) }).await;
}

/// Both Macs agreed on tone, then each changed it while Home's Gateway was down: held as a clash on both.
async fn clashed(home: &mut Kitted, work: &Kitted) -> (String, String) {
    home.wrote("tone", TONE).await;
    pair(home, work).await;
    let (home_id, work_id) = (home.mac.id.clone(), work.mac.id.clone());
    until("tone agreed on Work", || async { work.mark("tone", &home_id).await == Some(KitMark::Same) }).await;
    until("and on Home", || async { home.mark("tone", &work_id).await == Some(KitMark::Same) }).await;

    home.mac.stop().await;
    until("Home asleep on Work", || async {
        matches!(work.mac.mac(&home_id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. }))
    })
    .await;
    let (mine, theirs) = ("---\ndescription: house tone\n---\nHome's tone.\n".to_string(), "---\ndescription: house tone\n---\nWork's tone.\n".to_string());
    home.wrote("tone", &mine).await;
    work.wrote("tone", &theirs).await;
    home.mac.restart();
    until("held as a clash on Work", || async { work.mark("tone", &home_id).await == Some(KitMark::Clash) }).await;
    until("and on Home", || async { home.mark("tone", &work_id).await == Some(KitMark::Clash) }).await;
    (mine, theirs)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_edit_on_both_sides_is_held_as_a_clash_with_both_sides_and_neither_moves() {
    let (mut home, work) = linked(true, true).await;
    let (mine, theirs) = clashed(&mut home, &work).await;

    let on_work = work.against("tone", &home.mac.id).await.expect("tone against Home");
    let (here, there) = (on_work.here.expect("Work's side"), on_work.there.expect("Home's side"));
    // Each side named as its own Fleet names its Mac.
    let (work_name, home_name) = (work.mac.rig.fleet.kit_offered().await.expect("Work's").name, home.mac.rig.fleet.kit_offered().await.expect("Home's").name);
    assert_eq!((here.mac_name.as_str(), here.text.as_str()), (work_name.as_str(), theirs.as_str()));
    assert_eq!((there.mac_name.as_str(), there.text.as_str()), (home_name.as_str(), mine.as_str()));
    assert!(here.changed_at.is_some() && there.changed_at.is_some(), "each side says when its Mac first read it");
    assert_eq!(work.text("tone").as_deref(), Some(theirs.as_str()), "Work keeps its own");
    assert_eq!(home.text("tone").as_deref(), Some(mine.as_str()), "Home keeps its own");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn keeping_this_macs_or_the_other_macs_settles_a_clash_on_both_macs() {
    let (mut home, mut work) = linked(true, true).await;
    let (home_id, work_id) = (home.mac.id.clone(), work.mac.id.clone());
    let tone = HomeItem { kind: HomeKind::Skills, name: "tone".into(), repository: None };
    let settle = |mac: &str, keep| SettleKitClash { mac: mac.to_string(), item: tone.clone(), keep };

    // 1. Keep the other Mac's, pressed on Home: Home takes Work's at once, and both read the same.
    let (_, theirs) = clashed(&mut home, &work).await;
    home.mac.rig.fleet.settle_kit_clash(settle(&work_id, KitKeep::There)).await.expect("settled");
    assert_eq!(home.text("tone").as_deref(), Some(theirs.as_str()), "Work's side written on Home at once");
    until("the same on Work", || async { work.mark("tone", &home_id).await == Some(KitMark::Same) }).await;
    until("and on Home", || async { home.mark("tone", &work_id).await == Some(KitMark::Same) }).await;

    // 2. A clash again; keep this Mac's, pressed on Home while Work's Gateway is down: queued, and when Work's link
    //    opens Work takes Home's.
    work.mac.stop().await;
    until("Work asleep on Home", || async {
        matches!(home.mac.mac(&work_id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. }))
    })
    .await;
    let (mine, again) = ("---\ndescription: house tone\n---\nHome's, kept.\n", "---\ndescription: house tone\n---\nWork's, dropped.\n");
    home.wrote("tone", mine).await;
    work.wrote("tone", again).await;
    work.mac.restart();
    until("held as a clash on Home", || async { home.mark("tone", &work_id).await == Some(KitMark::Clash) }).await;
    work.mac.stop().await;
    until("Work asleep on Home again", || async {
        matches!(home.mac.mac(&work_id).await.and_then(|mac| mac.link), Some(MacLink::Asleep { .. }))
    })
    .await;
    home.mac.rig.fleet.settle_kit_clash(settle(&work_id, KitKeep::Here)).await.expect("settled and queued");
    assert_eq!(work.text("tone").as_deref(), Some(again), "Work has not heard it yet");
    work.mac.restart();
    until("Home's side reaches Work", || async { work.text("tone").as_deref() == Some(mine) }).await;
    assert_eq!(home.text("tone").as_deref(), Some(mine), "Home kept its own");
    until("the same on both", || async {
        work.mark("tone", &home_id).await == Some(KitMark::Same) && home.mark("tone", &work_id).await == Some(KitMark::Same)
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_piece_holding_a_credential_stays_here_and_says_why_naming_the_host_and_never_the_value() {
    let (home, work) = linked(true, true).await;
    home.wrote("tone", TONE).await;
    home.wrote("keys", "---\ndescription: calls the API\n---\ncurl https://ana:hunter2@api.example.dev/v1\n").await;
    pair(&home, &work).await;
    until("tone reaches Work", || async { work.text("tone").is_some() }).await;

    let link = home.read().await;
    let keys = link.rows.iter().find(|row| row.name == "keys").expect("the row stays on Home");
    let why = keys.refused.as_deref().expect("it says why");
    assert!(why.contains("api.example.dev"), "{why}");
    assert!(!why.contains("hunter2") && !why.contains("ana:"), "never the value: {why}");
    let offer = home.mac.rig.fleet.kit_offered().await.expect("Home's offer");
    assert!(offer.pieces.iter().all(|piece| piece.name != "keys"), "not offered");
    assert_eq!(work.text("keys"), None, "never on Work");
    assert!(work.read().await.rows.iter().all(|row| row.name != "keys"), "Work does not know it exists");
}

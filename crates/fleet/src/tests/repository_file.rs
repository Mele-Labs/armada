//! `get_repository_file`: a Job's file read by Fleet for a client that cannot read the agent's disk.
//! Electron's file panel rules are the spec. **No terminal and no agent**: a session reported as
//! started in a directory the test makes, its ledger written by reports, and the file read through the
//! route's own trait method.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use api::{HostedSessions, Refusal, Sessions};
use base64::Engine as _;
use ipc::{AttachmentReport, RepositoryFile, SessionFact, SessionId, SessionOrigin, SessionReport};
use testkit::{FakeHarness, FakeVcs, FakeWorkProduct};

use super::session_host::{Shared, StandIn};
use crate::daemon::Fleet;
use crate::tests::daemon::a_fleet;
use crate::tests::tmp::TempDir;
use crate::session_host::repository_file::{
    REPOSITORY_FILE_BINARY as BINARY, REPOSITORY_FILE_NOT_ADDRESSABLE as NOT_ADDRESSABLE, REPOSITORY_FILE_TOO_BIG as TOO_BIG,
    REPOSITORY_FILE_UNREADABLE as UNREADABLE,
};

type Hosted = Fleet<FakeHarness, FakeVcs, FakeWorkProduct>;

const ID: &str = "rf-1111";

const MOST: u64 = 8 * 1024 * 1024;

/// A Job started in `checkout`, a folder inside `base`, with `outside` as another folder of `base` that
/// the Job's checkout does not hold.
struct Rig {
    fleet: Arc<Hosted>,
    checkout: std::path::PathBuf,
    outside: std::path::PathBuf,
    _base: TempDir,
    _home: TempDir,
}

async fn rig() -> Rig {
    let home = TempDir::new();
    let base = TempDir::new();
    let fleet = a_fleet(&home, FakeWorkProduct::changed(&[])).hosting_sessions_on(
        Arc::new(Shared(Arc::new(StandIn::default()))),
        Duration::from_secs(600),
    );
    let base_path = std::fs::canonicalize(base.path()).unwrap();
    let checkout = base_path.join("checkout");
    let outside = base_path.join("outside");
    std::fs::create_dir_all(&checkout).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let rig = Rig { fleet: Arc::new(fleet), checkout, outside, _base: base, _home: home };
    rig.reports(SessionFact::Started {
        cwd: rig.checkout.to_string_lossy().into_owned(),
        title: None,
        origin: SessionOrigin::Terminal,
        mod_version: None,
    })
    .await;
    rig
}

impl Rig {
    async fn reports(&self, fact: SessionFact) {
        self.fleet
            .report_session(SessionReport { harness: adapters::HOSTED_HARNESS.into(), session_id: SessionId::carried(ID), fact })
            .await
            .unwrap();
    }

    /// The ledger gains an `artifact` whose `form` and other detail are as given.
    async fn names(&self, target: &Path, detail: &[(&str, &str)]) {
        self.reports(SessionFact::Attached {
            attachment: AttachmentReport {
                kind: "artifact".into(),
                target: target.to_string_lossy().into_owned(),
                detail: detail.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
            },
        })
        .await;
    }

    async fn reads(&self, path: &Path) -> Result<RepositoryFile, Refusal> {
        self.reads_as(ID, &path.to_string_lossy()).await
    }

    async fn reads_as(&self, id: &str, path: &str) -> Result<RepositoryFile, Refusal> {
        self.fleet.get_repository_file(SessionId::carried(id), path.to_string()).await
    }

    /// The code and status of the refusal a read ends in.
    async fn refused(&self, path: &Path) -> (u16, String) {
        let refusal = self.reads(path).await.expect_err("refused");
        (refusal.status(), refusal.error().code.clone())
    }
}

fn text(text: &str, media_type: &str) -> RepositoryFile {
    RepositoryFile::Text { text: text.into(), media_type: media_type.into() }
}

fn picture(bytes: &[u8], media_type: &str) -> RepositoryFile {
    RepositoryFile::Picture { bytes: base64::engine::general_purpose::STANDARD.encode(bytes), media_type: media_type.into() }
}

#[tokio::test]
async fn repository_file_reads_text_in_the_checkout() {
    let rig = rig().await;
    std::fs::create_dir_all(rig.checkout.join("docs")).unwrap();
    std::fs::write(rig.checkout.join("README.md"), "# Hello\n").unwrap();
    std::fs::write(rig.checkout.join("docs/notes.txt"), "plain\n").unwrap();
    // Links followed: one that ends inside the checkout is inside it.
    std::os::unix::fs::symlink(rig.checkout.join("README.md"), rig.checkout.join("linked.md")).unwrap();

    assert_eq!(rig.reads(Path::new("README.md")).await.unwrap(), text("# Hello\n", "text/markdown"), "relative");
    assert_eq!(rig.reads(Path::new("docs/notes.txt")).await.unwrap(), text("plain\n", "text/plain"), "nested");
    assert_eq!(rig.reads(&rig.checkout.join("README.md")).await.unwrap(), text("# Hello\n", "text/markdown"), "absolute");
    assert_eq!(rig.reads(Path::new("docs/../README.md")).await.unwrap(), text("# Hello\n", "text/markdown"), "a `..` that stays inside");
    assert_eq!(rig.reads(Path::new("linked.md")).await.unwrap(), text("# Hello\n", "text/markdown"), "a link that ends inside");
}

#[tokio::test]
async fn repository_file_types_follow_the_extension() {
    let rig = rig().await;
    for (name, expected) in [
        ("a.markdown", "text/markdown"),
        ("b.MD", "text/markdown"),
        ("c.rs", "text/plain"),
        ("d", "text/plain"),
    ] {
        std::fs::write(rig.checkout.join(name), "x").unwrap();
        assert_eq!(rig.reads(Path::new(name)).await.unwrap(), text("x", expected), "{name}");
    }
    for (name, expected) in [
        ("e.png", "image/png"),
        ("f.jpg", "image/jpeg"),
        ("g.JPEG", "image/jpeg"),
        ("h.gif", "image/gif"),
        ("i.webp", "image/webp"),
        ("j.svg", "image/svg+xml"),
    ] {
        // A picture is not looked into: a NUL and bytes that are not UTF-8 are what a picture is made of.
        let bytes = [0u8, 0xff, 0xfe, 1];
        std::fs::write(rig.checkout.join(name), bytes).unwrap();
        assert_eq!(rig.reads(Path::new(name)).await.unwrap(), picture(&bytes, expected), "{name}");
    }
}

#[tokio::test]
async fn repository_file_reads_a_ledger_image_outside_it() {
    let rig = rig().await;
    let image = rig.outside.join("shot.png");
    std::fs::write(&image, [0x89, b'P', b'N', b'G', 0, 1, 2]).unwrap();
    std::fs::write(rig.outside.join("seen.txt"), "the agent wrote this\n").unwrap();
    std::fs::write(rig.outside.join("note.jpg"), [0xff, 0xd8]).unwrap();
    std::fs::write(rig.outside.join("page-1.html.txt"), "kept").unwrap();
    std::fs::write(rig.outside.join("draw-1.svg"), "<svg/>").unwrap();

    assert_eq!(rig.reads(&image).await.unwrap_err().error().code, NOT_ADDRESSABLE, "named by nothing yet");

    rig.names(&image, &[("form", "image")]).await;
    assert_eq!(rig.reads(&image).await.unwrap(), picture(&[0x89, b'P', b'N', b'G', 0, 1, 2], "image/png"), "an image artifact");

    let seen = rig.outside.join("seen.txt");
    rig.names(&seen, &[("form", "file")]).await;
    assert_eq!(rig.reads(&seen).await.unwrap(), text("the agent wrote this\n", "text/plain"), "a file artifact");

    let note = rig.outside.join("note.jpg");
    rig.names(Path::new("note:1"), &[("form", "note"), ("picture", &note.to_string_lossy())]).await;
    assert_eq!(rig.reads(&note).await.unwrap(), picture(&[0xff, 0xd8], "image/jpeg"), "a note's picture");

    let page = rig.outside.join("page-1.html.txt");
    let draw = rig.outside.join("draw-1.svg");
    let revisions = |one: &Path| format!(r#"["{}"]"#, one.display());
    rig.names(Path::new("/doc.html"), &[("form", "document"), ("revisions", &revisions(&page))]).await;
    rig.names(Path::new("sketch:a"), &[("form", "sketch"), ("revisions", &revisions(&draw))]).await;
    assert_eq!(rig.reads(&page).await.unwrap(), text("kept", "text/plain"), "a document's kept revision");
    assert_eq!(rig.reads(&draw).await.unwrap(), picture(b"<svg/>", "image/svg+xml"), "a sketch's kept revision");

    // A relative path the ledger names resolves against the Job's cwd, and may leave it.
    rig.names(Path::new("../outside/seen.txt"), &[("form", "file")]).await;
    assert_eq!(rig.reads(Path::new("../outside/seen.txt")).await.unwrap(), text("the agent wrote this\n", "text/plain"), "relative to the cwd");

    // What the ledger does not name stays unread, beside what it does.
    std::fs::write(rig.outside.join("other.txt"), "no").unwrap();
    assert_eq!(rig.refused(&rig.outside.join("other.txt")).await, (422, NOT_ADDRESSABLE.into()), "its neighbour");
}

#[tokio::test]
async fn repository_file_refuses_a_path_outside_the_checkout() {
    let rig = rig().await;
    std::fs::write(rig.outside.join("secret.txt"), "no").unwrap();
    std::fs::write(rig.checkout.join("fine.txt"), "yes").unwrap();
    std::fs::create_dir_all(rig.checkout.join("dir")).unwrap();
    std::os::unix::fs::symlink(rig.outside.join("secret.txt"), rig.checkout.join("out.txt")).unwrap();
    std::os::unix::fs::symlink(&rig.outside, rig.checkout.join("out_dir")).unwrap();
    // A sibling folder whose name starts with the checkout's is not inside it.
    let sibling = rig.checkout.with_file_name("checkout-and-more");
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join("x.txt"), "no").unwrap();

    let refused = (422, NOT_ADDRESSABLE.to_string());
    assert_eq!(rig.refused(Path::new("../outside/secret.txt")).await, refused, "a `..` that leaves");
    assert_eq!(rig.refused(&rig.outside.join("secret.txt")).await, refused, "an absolute path outside");
    assert_eq!(rig.refused(Path::new("out.txt")).await, refused, "a link to a file outside");
    assert_eq!(rig.refused(Path::new("out_dir/secret.txt")).await, refused, "a link to a folder outside");
    assert_eq!(rig.refused(&sibling.join("x.txt")).await, refused, "a sibling that shares a prefix");
    assert_eq!(rig.refused(Path::new("dir")).await, refused, "a folder");
    assert_eq!(rig.refused(Path::new(".")).await, refused, "the checkout itself");
    assert_eq!(rig.refused(Path::new("missing.txt")).await, refused, "a file that is not there");
    assert_eq!(rig.refused(Path::new("")).await, refused, "no path");
    assert!(rig.reads(Path::new("fine.txt")).await.is_ok(), "the rest of the checkout reads");
}

#[tokio::test]
async fn repository_file_refuses_too_big() {
    let rig = rig().await;
    std::fs::write(rig.checkout.join("big.txt"), vec![b'a'; MOST as usize + 1]).unwrap();
    std::fs::write(rig.checkout.join("most.txt"), vec![b'a'; MOST as usize]).unwrap();
    let big_picture = std::fs::File::create(rig.checkout.join("big.png")).unwrap();
    big_picture.set_len(MOST + 1).unwrap();

    assert_eq!(rig.refused(Path::new("big.txt")).await, (422, TOO_BIG.into()));
    assert_eq!(rig.refused(Path::new("big.png")).await, (422, TOO_BIG.into()), "a picture too");
    assert!(matches!(rig.reads(Path::new("most.txt")).await, Ok(RepositoryFile::Text { .. })), "8 MiB itself is read");
}

#[tokio::test]
async fn repository_file_refuses_binary() {
    let rig = rig().await;
    std::fs::write(rig.checkout.join("nul.dat"), b"ab\0cd").unwrap();
    std::fs::write(rig.checkout.join("latin1.txt"), [b'a', 0xe9, b'b']).unwrap();
    std::fs::write(rig.checkout.join("cut.txt"), [b'a', 0xe2, 0x82]).unwrap();
    // A NUL past the first 4096 bytes is not looked for, and it is valid UTF-8.
    let mut late = vec![b'a'; 4096];
    late.push(0);
    std::fs::write(rig.checkout.join("late.txt"), &late).unwrap();
    // The last of the 4096 is looked at.
    let mut last = vec![b'a'; 4095];
    last.push(0);
    std::fs::write(rig.checkout.join("last.txt"), &last).unwrap();

    assert_eq!(rig.refused(Path::new("nul.dat")).await, (422, BINARY.into()), "a NUL");
    assert_eq!(rig.refused(Path::new("latin1.txt")).await, (422, BINARY.into()), "not UTF-8");
    assert_eq!(rig.refused(Path::new("cut.txt")).await, (422, BINARY.into()), "a character cut short");
    assert_eq!(rig.refused(Path::new("last.txt")).await, (422, BINARY.into()), "a NUL at byte 4095");
    assert!(matches!(rig.reads(Path::new("late.txt")).await, Ok(RepositoryFile::Text { .. })), "a NUL at byte 4096");
}

#[tokio::test]
async fn repository_file_refusals_in_order() {
    let rig = rig().await;

    // No such Job comes first, whatever the path.
    let none = rig.reads_as("no-such-job", "README.md").await.unwrap_err();
    assert_eq!((none.status(), none.error().code.as_str()), (422, NOT_ADDRESSABLE), "no record");

    // Not addressable before too big: a file outside that is also over the limit.
    let huge_outside = std::fs::File::create(rig.outside.join("huge.txt")).unwrap();
    huge_outside.set_len(MOST + 1).unwrap();
    assert_eq!(rig.refused(&rig.outside.join("huge.txt")).await, (422, NOT_ADDRESSABLE.into()), "outside and too big");

    // Too big before binary: nine MiB of NULs.
    let huge_nul = std::fs::File::create(rig.checkout.join("huge.bin")).unwrap();
    huge_nul.set_len(MOST + 1).unwrap();
    assert_eq!(rig.refused(Path::new("huge.bin")).await, (422, TOO_BIG.into()), "too big and binary");

    // Binary before anything is drawn; a NUL and bad UTF-8 together are one refusal.
    std::fs::write(rig.checkout.join("both.bin"), [0, 0xff]).unwrap();
    assert_eq!(rig.refused(Path::new("both.bin")).await, (422, BINARY.into()), "a NUL and not UTF-8");

    // Unreadable is any other failure, reached only for a path a rule placed: a file the ledger names that is
    // gone, a folder it names, and a file the Job may not open.
    let gone = rig.outside.join("gone.png");
    rig.names(&gone, &[("form", "image")]).await;
    assert_eq!(rig.refused(&gone).await, (500, UNREADABLE.into()), "named and gone");
    rig.names(&rig.outside, &[("form", "file")]).await;
    assert_eq!(rig.refused(&rig.outside).await, (500, UNREADABLE.into()), "named and a folder");
    let locked = rig.checkout.join("locked.txt");
    std::fs::write(&locked, "x").unwrap();
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let opened_anyway = std::fs::File::open(&locked).is_ok();
    if !opened_anyway {
        assert_eq!(rig.refused(Path::new("locked.txt")).await, (500, UNREADABLE.into()), "in the checkout and closed");
    }
}

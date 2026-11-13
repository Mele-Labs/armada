//! What Fleet remembers of a pull request for the person between visits, and what it works out
//! from it: the head he last left it at, the files he marked viewed and whether each still is,
//! where each row of the lists stands in his inbox, the change since he looked, and his named
//! Reviews layouts. `docs/concepts/pull-review.md`, *Since you looked*.

use std::collections::HashMap;

use adapter_traits::{AgentHarness, ComparedPatches, Delivery, LatestReview, LatestReviewSaid, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    ManifestId, MarkFileViewed, MarkPullSeen, MyReviewSaid, NamedReviewLayout, PullInbox, PullMyReview,
    PullRequestCompare, PullRequestCompareOf, PullRequestPage, PullReviewRow, PullSeen, PullViewedFile, ReviewLayout,
    ReviewLayouts, SaveReviewLayout, UnmarkFileViewed, DEFAULT_REVIEW_LAYOUT,
};
use store::{KeptReviewLayout, KeptSeenHead, KeptViewedFile};

use crate::daemon::Fleet;
use crate::pull_request_panel::file_patch_of;

/// A mark, a compare or a layout that cannot be taken. A 422.
pub(crate) const MEMORY_INVALID: &str = "fleet.pull_review_memory_invalid";
/// The forge would not compare the two commits. A 500.
const COMPARE_UNREADABLE: &str = "fleet.pull_request_compare_unreadable";

/// The longest a layout's name may be, in characters.
const LAYOUT_NAME_AT_MOST: usize = 60;
/// The most an arrangement may hold, in bytes. A pane tree is a few kilobytes at most.
const ARRANGEMENT_AT_MOST: usize = 64 * 1024;

/// Fleet's own layout: three columns, the inbox | what changed, the files, the conversation and the
/// checks | the findings and the comments.
pub(crate) const DEFAULT_ARRANGEMENT: &str = include_str!("pull_review_memory/default_layout.json");

/// Where a row stands in his inbox. **The first that holds wins**: a head moved since he looked,
/// then his latest review approving, then it commenting or asking for changes, then anything else.
pub(crate) fn inbox_of(seen: Option<&PullSeen>, head: &str, mine: Option<&PullMyReview>) -> PullInbox {
    if seen.is_some_and(|seen| !head.is_empty() && seen.head != head) {
        return PullInbox::ChangedSinceYouLooked;
    }
    match mine.map(|mine| mine.said) {
        Some(MyReviewSaid::Approved) => PullInbox::Done,
        Some(MyReviewSaid::Commented | MyReviewSaid::ChangesRequested) => PullInbox::WaitingOnAuthor,
        None => PullInbox::NeedsFirstLook,
    }
}

/// His latest review as the wire carries it.
pub(crate) fn my_review_of(latest: LatestReview) -> PullMyReview {
    PullMyReview {
        said: match latest.said {
            LatestReviewSaid::Approved => MyReviewSaid::Approved,
            LatestReviewSaid::ChangesRequested => MyReviewSaid::ChangesRequested,
            LatestReviewSaid::Commented => MyReviewSaid::Commented,
        },
        commit: latest.commit,
    }
}

/// Whether a file he marked viewed still is: **it has not changed since**. Viewed at the head, it
/// is. Otherwise the forge's blob for it at the head must be the one he viewed; where he kept none,
/// or the forge gave none, the head moving is a change.
pub(crate) fn still_viewed(kept: &KeptViewedFile, head: Option<&str>, blobs: &HashMap<String, Option<String>>) -> bool {
    if head == Some(kept.commit.as_str()) {
        return true;
    }
    match (&kept.blob, blobs.get(&kept.path)) {
        (Some(viewed), Some(Some(now))) => viewed == now,
        _ => false,
    }
}

/// A whole commit name as the forge writes one: hex, at least seven characters, at most 64.
fn is_commit(name: &str) -> bool {
    (7..=64).contains(&name.len()) && name.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn seen_of(kept: KeptSeenHead) -> PullSeen {
    PullSeen { head: kept.head, at: kept.seen_at }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    fn memory_invalid(&self, said: impl Into<String>) -> Refusal {
        Refusal::Unacceptable(self.pr_refusal(MEMORY_INVALID, said))
    }

    fn memory_unkept(&self, why: store::WriteError) -> Refusal {
        self.refusal(crate::adrift::Adrift::Writing(why))
    }

    /// The served repository's root, and its name as the forge writes it, lowercase, which is how
    /// the lists name it too.
    async fn remembered_repository(&self, manifest_id: &ManifestId) -> Result<(String, String), Refusal> {
        let served = self.served_named(Some(manifest_id))?;
        let slug = self.walk_slug(served.root(), manifest_id).await.to_lowercase();
        Ok((served.root().to_string(), slug))
    }

    /// Each row's seen head and inbox state, from one read of the seen heads. Unreadable, every row
    /// stands as his review alone says.
    pub(crate) async fn inbox_onto(&self, rows: &mut [PullReviewRow]) {
        let kept = self.store().lock().await.seen_heads().unwrap_or_default();
        let seen: HashMap<(&str, String, u64), &KeptSeenHead> = kept
            .iter()
            .map(|it| ((it.manifest_id.as_str(), it.repository.to_lowercase(), it.number), it))
            .collect();
        for row in rows {
            let key = (row.manifest_id.as_str(), row.repository.to_lowercase(), row.number);
            row.seen = seen.get(&key).map(|it| seen_of((*it).clone()));
            row.inbox = inbox_of(row.seen.as_ref(), &row.head, row.my_review.as_ref());
        }
    }

    /// The page with his seen head and the files he marked viewed. **The forge's files are read only
    /// where a mark is at an older head**, for their blobs. Memory that cannot be read leaves the
    /// page as the forge gave it.
    pub(crate) async fn remembered_onto(&self, page: &mut PullRequestPage, root: &str) {
        let slug = self.walk_slug(root, &page.manifest_id).await.to_lowercase();
        let (seen, viewed) = {
            let store = self.store().lock().await;
            let seen = store.seen_head(page.manifest_id.as_str(), &slug, page.number).ok().flatten();
            let viewed = store.viewed_files(page.manifest_id.as_str(), &slug, page.number).unwrap_or_default();
            (seen, viewed)
        };
        page.seen = seen.map(seen_of);
        let head = page.head.clone();
        let behind = viewed.iter().any(|it| head.as_deref() != Some(it.commit.as_str()) && it.blob.is_some());
        let blobs: HashMap<String, Option<String>> = if behind {
            let number = page.number.to_string();
            self.forge_asked(root, move |vcs: &V, root| vcs.pull_request_patches(root, &number))
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|file| (file.path, file.blob))
                .collect()
        } else {
            HashMap::new()
        };
        page.viewed = viewed
            .iter()
            .map(|kept| PullViewedFile {
                path: kept.path.clone(),
                commit: kept.commit.clone(),
                viewed_at: kept.viewed_at.clone(),
                still: still_viewed(kept, head.as_deref(), &blobs),
            })
            .collect();
    }

    /// Keep the head he left a pull request at.
    pub(crate) async fn pull_seen_marked(&self, manifest_id: ManifestId, mark: MarkPullSeen) -> Result<PullSeen, Refusal> {
        if !is_commit(&mark.head) {
            return Err(self.memory_invalid(format!("{} is not a commit", mark.head)));
        }
        let (_, repository) = self.remembered_repository(&manifest_id).await?;
        let kept = KeptSeenHead {
            manifest_id: manifest_id.as_str().to_string(),
            repository,
            number: mark.number,
            head: mark.head,
            seen_at: self.clock().now().as_str().to_string(),
        };
        self.store().lock().await.keep_seen_head(&kept).map_err(|why| self.memory_unkept(why))?;
        Ok(seen_of(kept))
    }

    /// Keep a file as viewed at a commit, with the forge's blob for it there where Bridge has it.
    pub(crate) async fn file_viewed_marked(&self, manifest_id: ManifestId, mark: MarkFileViewed) -> Result<PullViewedFile, Refusal> {
        if mark.path.trim().is_empty() {
            return Err(self.memory_invalid("a file to mark viewed needs a path"));
        }
        if !is_commit(&mark.commit) {
            return Err(self.memory_invalid(format!("{} is not a commit", mark.commit)));
        }
        let (_, repository) = self.remembered_repository(&manifest_id).await?;
        let kept = KeptViewedFile {
            manifest_id: manifest_id.as_str().to_string(),
            repository,
            number: mark.number,
            path: mark.path,
            commit: mark.commit,
            blob: mark.blob.filter(|blob| !blob.is_empty()),
            viewed_at: self.clock().now().as_str().to_string(),
        };
        self.store().lock().await.keep_viewed_file(&kept).map_err(|why| self.memory_unkept(why))?;
        Ok(PullViewedFile { path: kept.path, commit: kept.commit, viewed_at: kept.viewed_at, still: true })
    }

    /// Forget a file's viewed mark. One not marked is not an error.
    pub(crate) async fn file_viewed_unmarked(&self, manifest_id: ManifestId, unmark: UnmarkFileViewed) -> Result<UnmarkFileViewed, Refusal> {
        let (_, repository) = self.remembered_repository(&manifest_id).await?;
        self.store()
            .lock()
            .await
            .forget_viewed_file(manifest_id.as_str(), &repository, unmark.number, &unmark.path)
            .map_err(|why| self.memory_unkept(why))?;
        Ok(unmark)
    }

    /// What changed from `since` to `head`, as the forge compares them: **from their merge base**.
    pub(crate) async fn pull_request_compared(
        &self,
        manifest_id: ManifestId,
        number: u64,
        of: PullRequestCompareOf,
    ) -> Result<PullRequestCompare, Refusal> {
        for named in [&of.since, &of.head] {
            if !is_commit(named) {
                return Err(self.memory_invalid(format!("{named} is not a commit")));
            }
        }
        let served = self.served_named(Some(&manifest_id))?;
        let (since, head) = (of.since.clone(), of.head.clone());
        let compared = self
            .forge_asked(served.root(), move |vcs: &V, root| vcs.pull_request_compare(root, &since, &head))
            .await
            .ok_or_else(|| {
                Refusal::Fault(self.pr_refusal(
                    COMPARE_UNREADABLE,
                    format!("the forge would not compare {} with {}", of.since, of.head),
                ))
            })?;
        Ok(match compared {
            ComparedPatches::Patches(files) => PullRequestCompare::Patches {
                files: files.into_iter().map(|file| file_patch_of(&manifest_id, number, file)).collect(),
                since: of.since,
                head: of.head,
            },
            ComparedPatches::BaseGone => PullRequestCompare::BaseGone { since: of.since, head: of.head },
        })
    }

    /// Every layout, Fleet's first, and the one picked: Default where he never picked or the one he
    /// picked is gone.
    pub(crate) async fn review_layouts_listed(&self) -> Result<ReviewLayouts, Refusal> {
        let store = self.store().lock().await;
        let kept = store.review_layouts().map_err(|why| self.memory_unkept(why))?;
        let picked = store.picked_review_layout().map_err(|why| self.memory_unkept(why))?;
        let mut layouts = vec![ReviewLayout {
            name: DEFAULT_REVIEW_LAYOUT.to_string(),
            arrangement: DEFAULT_ARRANGEMENT.trim().to_string(),
            shipped: true,
        }];
        layouts.extend(kept.into_iter().map(|it| ReviewLayout { name: it.name, arrangement: it.arrangement, shipped: false }));
        let picked = picked
            .filter(|name| layouts.iter().any(|it| it.name == *name))
            .unwrap_or_else(|| DEFAULT_REVIEW_LAYOUT.to_string());
        Ok(ReviewLayouts { layouts, picked })
    }

    /// A name he may write: not blank, not too long, and not Fleet's own.
    fn layout_name(&self, name: &str) -> Result<String, Refusal> {
        let name = name.trim();
        if name.is_empty() {
            return Err(self.memory_invalid("a layout needs a name"));
        }
        if name.chars().count() > LAYOUT_NAME_AT_MOST {
            return Err(self.memory_invalid(format!("a layout's name is at most {LAYOUT_NAME_AT_MOST} characters")));
        }
        if name.eq_ignore_ascii_case(DEFAULT_REVIEW_LAYOUT) {
            return Err(self.memory_invalid(format!("{DEFAULT_REVIEW_LAYOUT} is Fleet's own, so save yours under another name")));
        }
        Ok(name.to_string())
    }

    /// Keep a layout under a name, replacing one of that name. **Fleet reads no further into the
    /// arrangement than that it is a JSON object with a root**: the pane tree is Bridge's.
    pub(crate) async fn review_layout_saved(&self, save: SaveReviewLayout) -> Result<ReviewLayouts, Refusal> {
        let name = self.layout_name(&save.name)?;
        if save.arrangement.len() > ARRANGEMENT_AT_MOST {
            return Err(self.memory_invalid(format!("a layout is at most {ARRANGEMENT_AT_MOST} bytes")));
        }
        let read = ipc::decode::<HashMap<String, serde::de::IgnoredAny>>("an arrangement", save.arrangement.as_bytes());
        if !read.is_ok_and(|keys| keys.contains_key("root")) {
            return Err(self.memory_invalid("a layout's arrangement is a JSON object with a root"));
        }
        let kept = KeptReviewLayout { name, arrangement: save.arrangement, saved_at: self.clock().now().as_str().to_string() };
        self.store().lock().await.keep_review_layout(&kept).map_err(|why| self.memory_unkept(why))?;
        self.review_layouts_listed().await
    }

    /// Forget a layout he saved. Picked, the pull requests open on Default after.
    pub(crate) async fn review_layout_deleted(&self, named: NamedReviewLayout) -> Result<ReviewLayouts, Refusal> {
        let name = self.layout_name(&named.name)?;
        self.store().lock().await.forget_review_layout(&name).map_err(|why| self.memory_unkept(why))?;
        self.review_layouts_listed().await
    }

    /// Name the layout every pull request opens on.
    pub(crate) async fn review_layout_picked(&self, named: NamedReviewLayout) -> Result<ReviewLayouts, Refusal> {
        let listed = self.review_layouts_listed().await?;
        if !listed.layouts.iter().any(|it| it.name == named.name) {
            return Err(self.memory_invalid(format!("there is no layout named {}", named.name)));
        }
        self.store().lock().await.pick_review_layout(&named.name).map_err(|why| self.memory_unkept(why))?;
        self.review_layouts_listed().await
    }
}

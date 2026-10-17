//! Which of the four refusals a failure is, and the code it carries.
//!
//! Split from [`serving`](mod@crate::serving) at the 900-line refusal, and
//! along the seam that file's own header already names: the trait impl decides
//! *what* to answer, and this decides *how a failure is spelled on the wire*.
//! Every `WireError` Fleet raises is raised here.
//!
//! **The choice is made from the typed leaf error and never from a message.**
//! `api::Refusal` has four variants because a caller has four things to do
//! about them — retry, fix the request, look somewhere else, or report a fault
//! — and a string match would make that mapping depend on wording.
//!
//! **The codes are declared here rather than in a registry**, beside the thing
//! that raises them: the set is closed by collection, and a central list would
//! put every code far from the failure it names.
//!

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{RunId, WireError, WireValue};
use store::WriteError;

use crate::adrift::Adrift;
use crate::daemon::Fleet;

/// The codes this boundary raises, declared beside the thing that raises them.
///
/// The set is closed by collection rather than by authorship — a central
/// registry would put every code far from the failure it names.
const FAULT: &str = "fleet.fault";
/// A Manifest this Fleet does not serve. A 422.
pub(crate) const NOT_SERVED: &str = "fleet.repository_not_served";
/// A `save_preferences` naming something outside the five preferences it
/// reads. A 422, refused by name — `name` rides on the field the message quotes.
const UNKNOWN_PREFERENCE: &str = "fleet.unknown_preference";
/// A request that decoded and names something that cannot work.
const UNACCEPTABLE: &str = "fleet.unacceptable_proposal";
/// Main is not red, or a Session already has this red. A 409.
const MAIN_NOT_RED: &str = "fleet.main_not_red";
/// A newer commit's CI is running on main, so a red is held and not handed to a Session. A 409.
const MAIN_CHECKS_RUNNING: &str = "fleet.main_checks_running";
const MAIN_ALREADY_TAKEN: &str = "fleet.main_already_taken";
/// Another machine's Session holds this red in the central store. A 409, and
/// `take_over` is the answer to it.
const MAIN_TAKEN_ELSEWHERE: &str = "fleet.main_taken_elsewhere";
/// The central store could not be asked or said no, so the red was not claimed
/// and not taken. A 500: the store may answer on the next press.
const MAIN_CLAIM_NOT_MADE: &str = "fleet.main_claim_not_made";
/// The three Manifest-file codes. Why each is its own is `save_manifest_file`'s row.
const MANIFEST_UNREADABLE: &str = "fleet.manifest_unreadable";
const MANIFEST_UNWRITABLE: &str = "fleet.manifest_unwritable";
const MANIFEST_MOVED: &str = "fleet.manifest_moved_under_the_edit";
/// The base branch is protected. **Its own code, and the whole reason the
/// refusal kinds are not one**: what answers this is an administrator or a
/// rule, and nothing a person does to the pull request changes it.
pub(crate) const MERGE_BRANCH_PROTECTED: &str = "fleet.merge_branch_protected";
/// The branch and its base disagree. Its own code because the answer is on the
/// branch and never on the forge.
pub(crate) const MERGE_CONFLICTED: &str = "fleet.merge_conflicted";
/// A check the **forge** requires has not passed. Its own code because the
/// answer is to wait or to fix that check.
pub(crate) const MERGE_CHECKS_NOT_PASSED: &str = "fleet.merge_checks_not_passed";
/// The pull request is closed, gone, or was never there.
pub(crate) const MERGE_NOT_OPEN: &str = "fleet.merge_not_open";
/// Nothing on this machine could ask the forge. **A 500**, unlike the four
/// above: nothing about the request is wrong and asking again is reasonable
/// once whoever runs Fleet has signed in.
pub(crate) const MERGE_NO_TOOL: &str = "fleet.merge_no_tool";
/// The forge refused and said something this vocabulary has no name for. A 500
/// for [`MERGE_NO_TOOL`]'s reason, and the sentence the forge printed is in the
/// message — which is the honest answer where a guess would send a person to
/// fix the wrong thing.
pub(crate) const MERGE_REFUSED: &str = "fleet.merge_refused";

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
    /// Which of the refusals a failure is, decided from its type.
    ///
    /// **`run_id` names this process**, not Fleet's-by-assumption: it is minted
    /// where the emitter is, which is here.
    pub(crate) fn refusal(&self, why: Adrift) -> Refusal {
        let said = why.to_string();
        match &why {
            // A preference save named something outside the closed set. The
            // field is what says which name was refused, since `said` alone
            // does not travel structured.
            Adrift::Writing(WriteError::UnknownPreference { name }) => Refusal::Unacceptable(
                WireError::raised(UNKNOWN_PREFERENCE, said, self.run_id())
                    .with_field("name", WireValue::Str(name.clone())),
            ),
            Adrift::MainNotFixable { why, .. } => {
                use crate::main_fix::MainNotFixable as Why;
                let raised = |code| WireError::raised(code, said, self.run_id());
                match why {
                    Why::NotServed => Refusal::Unacceptable(raised(NOT_SERVED)),
                    Why::NotRed => Refusal::IllegalMove(raised(MAIN_NOT_RED)),
                    Why::ChecksRunning => Refusal::IllegalMove(raised(MAIN_CHECKS_RUNNING)),
                    Why::Taken(_) => Refusal::IllegalMove(raised(MAIN_ALREADY_TAKEN)),
                    Why::TakenElsewhere { .. } => Refusal::IllegalMove(raised(MAIN_TAKEN_ELSEWHERE)),
                    Why::ClaimNotMade(_) => Refusal::Fault(raised(MAIN_CLAIM_NOT_MADE)),
                }
            }
            // The request is well-formed and the values in it cannot work. Not
            // a 500: retrying it will fail identically forever, and the message
            // names what to send instead.
            Adrift::NotServed { .. } => {
                Refusal::Unacceptable(WireError::raised(NOT_SERVED, said, self.run_id()))
            }
            Adrift::NoSuchManifest { .. } | Adrift::NoSuchLandLog { .. } => {
                Refusal::Unacceptable(WireError::raised(UNACCEPTABLE, said, self.run_id()))
            }
            // Fleet's own ground failing; a refused parse is `ManifestReading`.
            Adrift::ManifestUnreadable { .. } => {
                Refusal::Fault(WireError::raised(MANIFEST_UNREADABLE, said, self.run_id()))
            }
            Adrift::ManifestUnwritable { .. } => {
                Refusal::Fault(WireError::raised(MANIFEST_UNWRITABLE, said, self.run_id()))
            }
            // A conflict, not a fault; the disk rides it, absent where it is gone.
            Adrift::ManifestMovedUnderTheEdit { on_disk, .. } => {
                let refused = WireError::raised(MANIFEST_MOVED, said, self.run_id());
                Refusal::IllegalMove(match on_disk {
                    Some(text) => refused.with_field("on_disk", WireValue::Str(text.clone())),
                    None => refused,
                })
            }
            Adrift::Reading(_) | Adrift::Writing(_) => {
                Refusal::Fault(WireError::raised(FAULT, said, self.run_id()))
            }
        }
    }

    /// This process's run id.
    ///
    /// **Not minted here per call.** It is the id of the emitter, and the
    /// emitter is one process — so it is derived from the mint once and held,
    /// which is what `run_id` names.
    pub(crate) fn run_id(&self) -> RunId {
        RunId::carried(self.run().as_str())
    }
}

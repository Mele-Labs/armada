//! A pull request by repository and number, and the acts a Session takes on
//! one: read it, take it out of draft, merge it, ask the forge to merge it when
//! its checks pass, and draft a Code Review Job against it. Since 23.48.
//! `docs/concepts/session.md`, *Acts on a pull request*.
//!
//! **Nobody's Job.** `merge_pull_request` is a Job's press at its gate and
//! records the landing on the Job. A Session's pull request has no Job to
//! record on, so these go to the forge through the same adapter calls and
//! answer the pull request as the forge shows it afterwards.
//!
//! **Every write is read first.** A refusal Fleet can name before it writes is
//! named, so a merge on a pull request whose checks are running never reaches
//! the forge, and the forge's own words are for what Fleet could not know.
//!
//! **The ledger follows the forge, through the ledger's own attach.** Every
//! read and every act refreshes the `pr` rows Sessions hold for the pull
//! request by reporting an `attached` fact for each, and settles a merged one
//! `spent` and a closed one `given_back`. Nothing here writes a ledger row
//! another way.

use std::collections::BTreeMap;

use adapter_traits::{
    AgentHarness, Delivery, NotMerged, PullRequestStanding as Forge, Vcs, WhatTheForgeRan,
    WorkProduct,
};
use api::{PullRequests, Redirector, Refusal, Sessions};
use ipc::{
    AttachmentNamed, AttachmentReport, AttachmentState, ForgeChecks, ManifestId,
    PullRequestStanding, PullRequestState, ReviewDispatched, ReviewPullRequest, SessionFact,
    SessionId, SessionReport, WireError,
};
use store::HolderKind;

use crate::daemon::Fleet;
use crate::refusing::{
    MERGE_BRANCH_PROTECTED, MERGE_CHECKS_NOT_PASSED, MERGE_CONFLICTED, MERGE_NOT_OPEN,
    MERGE_NO_TOOL, MERGE_REFUSED,
};

/// The forge would not say what the pull request is: no tool, nobody signed in,
/// no such number, or a state this build has no word for. A 500, because
/// asking again is reasonable.
const UNREADABLE: &str = "fleet.pull_request_unreadable";
/// Something that is neither a number nor a pull request's address. A 422.
const UNNAMED: &str = "fleet.pull_request_unnamed";
/// A draft cannot be merged, or have auto-merge asked of it. A 409.
const IS_A_DRAFT: &str = "fleet.pull_request_is_a_draft";
/// Taking out of draft what is not one. A 409.
const NOT_A_DRAFT: &str = "fleet.pull_request_not_a_draft";
/// Auto-merge asked for once every check has passed: merge it. A 409.
const CHECKS_PASSED: &str = "fleet.pull_request_checks_passed";
/// The forge would not take it out of draft, in its own sentence. A 409.
const READY_REFUSED: &str = "fleet.ready_refused";
/// The forge would not turn auto-merge on, in its own sentence. A 409.
const AUTO_MERGE_REFUSED: &str = "fleet.auto_merge_refused";
/// A review pressed from a Session the ledger has never heard of. A 422.
const SESSION_UNKNOWN: &str = "fleet.session_unknown";

/// The workflow a review runs on, named here and never left to the proposer.
const CODE_REVIEW: &str = "code_review";

/// The ledger's kind for a pull request a Session holds.
const PR: &str = "pr";

/// What a pull request was named as.
enum Named {
    Number(u64),
    Address { number: u64, address: String },
}

/// A number, or an address ending `/pull/<number>`.
fn named(said: &str) -> Option<Named> {
    let said = said.trim().trim_end_matches('/');
    if let Ok(number) = said.parse() {
        return Some(Named::Number(number));
    }
    let (before, last) = said.rsplit_once('/')?;
    let number = last.parse().ok()?;
    (before.ends_with("/pull") && before.starts_with("http")).then(|| Named::Address {
        number,
        address: said.to_string(),
    })
}

fn checks_of(ran: &WhatTheForgeRan) -> Option<ForgeChecks> {
    match ran {
        // **Not a pass**, `WhatTheForgeRan::NothingRan`'s own rule.
        WhatTheForgeRan::NothingRan | WhatTheForgeRan::StillWaiting { .. } => {
            Some(ForgeChecks::Pending)
        }
        WhatTheForgeRan::AllPassed { .. } => Some(ForgeChecks::Passed),
        WhatTheForgeRan::SomeFailed { failed, .. } => Some(ForgeChecks::Failed {
            failing: failed
                .iter()
                .map(|name| name.as_written().to_string())
                .collect(),
        }),
        WhatTheForgeRan::Unreadable => None,
    }
}

/// What a ledger row's `detail` says of the pull request, in the words the
/// wire uses.
fn detail_of(state: &PullRequestState) -> BTreeMap<String, String> {
    let (checks, failing) = match &state.checks {
        ForgeChecks::Pending => ("pending", String::new()),
        ForgeChecks::Passed => ("passed", String::new()),
        ForgeChecks::Failed { failing } => ("failed", failing.join(", ")),
    };
    let standing = match state.state {
        PullRequestStanding::Draft => "draft",
        PullRequestStanding::Open => "open",
        PullRequestStanding::Merged => "merged",
        PullRequestStanding::Closed => "closed",
    };
    BTreeMap::from([
        ("state".to_string(), standing.to_string()),
        ("auto_merge".to_string(), state.auto_merge.to_string()),
        ("checks".to_string(), checks.to_string()),
        ("failing".to_string(), failing),
        ("title".to_string(), state.title.clone()),
        ("branch".to_string(), state.branch.clone()),
        ("address".to_string(), state.address.clone()),
    ])
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
    fn pr_refusal(&self, code: &'static str, message: impl Into<String>) -> WireError {
        WireError::raised(code, message, self.run_id())
    }

    /// The pull request as the forge shows it, or the forge's silence. **Two
    /// asks, and either one silent is silent**: a state with no checks would
    /// let a merge be pressed on a guess.
    fn pull_request_read(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let named = number.to_string();
        let unreadable = || {
            Refusal::Fault(self.pr_refusal(
                UNREADABLE,
                format!("the forge would not say what pull request {number} is"),
            ))
        };
        let facts = self
            .vcs()
            .pull_request_facts(root, &named)
            .ok_or_else(unreadable)?;
        let checks = checks_of(&self.vcs().under_review(root, &named).checks)
            .ok_or_else(unreadable)?;
        Ok(PullRequestState {
            manifest_id: manifest_id.clone(),
            number,
            state: match facts.standing {
                Forge::Draft => PullRequestStanding::Draft,
                Forge::Open => PullRequestStanding::Open,
                Forge::Merged => PullRequestStanding::Merged,
                Forge::Closed => PullRequestStanding::Closed,
            },
            auto_merge: facts.auto_merge,
            checks,
            title: facts.title,
            branch: facts.branch,
            address: facts.url,
        })
    }

    /// Every Session still holding this pull request takes the state as the
    /// `detail` of its `pr` row, and a merged or closed one is settled.
    ///
    /// **Held, never raised**: the forge has already answered and the person
    /// asked about the pull request, not about the ledger, so a row that would
    /// not write is no reason to refuse an answer.
    async fn ledger_follows(&self, state: &PullRequestState) {
        let target = state.number.to_string();
        let sessions: Vec<(String, String)> = {
            let store = self.store().lock().await;
            let Ok(rows) = store.attachments_at(PR, &target, Some(state.manifest_id.as_str()))
            else {
                return;
            };
            rows.iter()
                .filter(|row| {
                    row.holder.kind == HolderKind::Session
                        && row.state == store::AttachmentState::Standing
                })
                .filter_map(|row| {
                    let session = store.session(&row.holder.id).ok().flatten()?;
                    Some((session.id, session.harness))
                })
                .collect()
        };
        for (session_id, harness) in sessions {
            let report = |fact| SessionReport {
                harness: harness.clone(),
                session_id: SessionId::carried(session_id.clone()),
                fact,
            };
            let _ = self
                .report_session(report(SessionFact::Attached {
                    attachment: AttachmentReport {
                        kind: PR.to_string(),
                        target: target.clone(),
                        detail: detail_of(state),
                    },
                }))
                .await;
            let over = match state.state {
                PullRequestStanding::Merged => Some(AttachmentState::Spent),
                PullRequestStanding::Closed => Some(AttachmentState::GivenBack),
                _ => None,
            };
            if let Some(over) = over {
                let _ = self
                    .report_session(report(SessionFact::Settled {
                        attachment: AttachmentNamed {
                            kind: PR.to_string(),
                            target: target.clone(),
                        },
                        state: over,
                    }))
                    .await;
            }
        }
    }

    /// Read, and let the ledger follow. Every act's last step and the query.
    async fn pull_request_followed(
        &self,
        manifest_id: &ManifestId,
        root: &str,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let state = self.pull_request_read(manifest_id, root, number)?;
        self.ledger_follows(&state).await;
        Ok(state)
    }

    fn not_merged_refusal(&self, number: u64, why: NotMerged) -> Refusal {
        let said = why.said();
        let raised = |code: &'static str| {
            self.pr_refusal(code, said.clone())
                .with_field("number", ipc::WireValue::Str(number.to_string()))
                .with_field("refused", ipc::WireValue::Str(why.kind().to_string()))
        };
        match &why {
            NotMerged::Protected { .. } => Refusal::IllegalMove(raised(MERGE_BRANCH_PROTECTED)),
            NotMerged::Conflicted { .. } => Refusal::IllegalMove(raised(MERGE_CONFLICTED)),
            NotMerged::ChecksNotPassed { .. } => {
                Refusal::IllegalMove(raised(MERGE_CHECKS_NOT_PASSED))
            }
            NotMerged::NotOpen { .. } => Refusal::IllegalMove(raised(MERGE_NOT_OPEN)),
            NotMerged::NoTool { .. } => Refusal::Fault(raised(MERGE_NO_TOOL)),
            // The four only `merge_by: push` says, and the need that holds a
            // Job's press: none of them is a forge merge's answer.
            NotMerged::BaseMoved { .. }
            | NotMerged::GateFailed { .. }
            | NotMerged::Unchecked { .. }
            | NotMerged::WaitingBehind { .. }
            | NotMerged::Refused { .. } => Refusal::Fault(raised(MERGE_REFUSED)),
        }
    }

    /// A pull request that is not open, as a refusal. `None` where it is.
    fn not_open_refusal(&self, state: &PullRequestState) -> Option<Refusal> {
        let raised = |code: &'static str, said: &str| {
            Refusal::IllegalMove(
                self.pr_refusal(code, format!("pull request {} {said}", state.number)),
            )
        };
        match state.state {
            PullRequestStanding::Draft => Some(raised(IS_A_DRAFT, "is a draft, and the forge will not do that to one")),
            PullRequestStanding::Merged => Some(raised(MERGE_NOT_OPEN, "is merged already")),
            PullRequestStanding::Closed => Some(raised(MERGE_NOT_OPEN, "is closed")),
            PullRequestStanding::Open => None,
        }
    }

    /// What a check that has not passed is called to a person.
    fn checks_not_passed(&self, state: &PullRequestState) -> Refusal {
        let said = match &state.checks {
            ForgeChecks::Failed { failing } => {
                format!("{} failed on pull request {}", failing.join(", "), state.number)
            }
            _ => format!("the checks on pull request {} are still running", state.number),
        };
        Refusal::IllegalMove(self.pr_refusal(MERGE_CHECKS_NOT_PASSED, said))
    }
}

impl<H, V, W> PullRequests for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_pull_request(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        self.pull_request_followed(&manifest_id, served.root(), number)
            .await
    }

    async fn ready_pull_request(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let root = served.root();
        let before = self.pull_request_read(&manifest_id, root, number)?;
        match before.state {
            PullRequestStanding::Draft => {}
            PullRequestStanding::Open => {
                return Err(Refusal::IllegalMove(self.pr_refusal(
                    NOT_A_DRAFT,
                    format!("pull request {number} is open already"),
                )))
            }
            _ => return Err(self.not_open_refusal(&before).expect("not open")),
        }
        self.vcs()
            .mark_ready(root, &number.to_string())
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(READY_REFUSED, said)))?;
        // A forge that took the write and would not answer after it is not a
        // refusal of the write.
        let after = PullRequestState {
            state: PullRequestStanding::Open,
            ..before
        };
        let state = self
            .pull_request_read(&manifest_id, root, number)
            .unwrap_or(after);
        self.ledger_follows(&state).await;
        Ok(state)
    }

    async fn merge_pull_request_by_number(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let root = served.root();
        let before = self.pull_request_read(&manifest_id, root, number)?;
        match before.state {
            // **Not a refusal**: the work is where the press was trying to put it.
            PullRequestStanding::Merged => {
                self.ledger_follows(&before).await;
                return Ok(before);
            }
            PullRequestStanding::Open => {}
            _ => return Err(self.not_open_refusal(&before).expect("not open")),
        }
        if before.checks != ForgeChecks::Passed {
            return Err(self.checks_not_passed(&before));
        }
        self.vcs()
            .merge(root, &number.to_string())
            .map_err(|why| self.not_merged_refusal(number, why))?;
        let after = PullRequestState {
            state: PullRequestStanding::Merged,
            ..before
        };
        let state = self
            .pull_request_read(&manifest_id, root, number)
            .unwrap_or(after);
        self.ledger_follows(&state).await;
        Ok(state)
    }

    async fn enable_auto_merge(
        &self,
        manifest_id: ManifestId,
        number: u64,
    ) -> Result<PullRequestState, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let root = served.root();
        let before = self.pull_request_read(&manifest_id, root, number)?;
        if let Some(refusal) = self.not_open_refusal(&before) {
            return Err(refusal);
        }
        // Asked twice, the second answers as it stands.
        if before.auto_merge {
            self.ledger_follows(&before).await;
            return Ok(before);
        }
        match &before.checks {
            ForgeChecks::Pending => {}
            ForgeChecks::Passed => {
                return Err(Refusal::IllegalMove(self.pr_refusal(
                    CHECKS_PASSED,
                    format!("the checks on pull request {number} have passed already: merge it"),
                )))
            }
            ForgeChecks::Failed { .. } => return Err(self.checks_not_passed(&before)),
        }
        self.vcs()
            .enable_auto_merge(root, &number.to_string())
            .map_err(|said| Refusal::IllegalMove(self.pr_refusal(AUTO_MERGE_REFUSED, said)))?;
        let after = PullRequestState {
            auto_merge: true,
            ..before
        };
        let state = self
            .pull_request_read(&manifest_id, root, number)
            .unwrap_or(after);
        self.ledger_follows(&state).await;
        Ok(state)
    }

    async fn review_pull_request(
        &self,
        manifest_id: ManifestId,
        review: ReviewPullRequest,
        by: Redirector,
    ) -> Result<ReviewDispatched, Refusal> {
        let served = self.served_named(Some(&manifest_id))?;
        let Some(asked) = named(&review.pull_request) else {
            return Err(Refusal::Unacceptable(self.pr_refusal(
                UNNAMED,
                format!(
                    "`{}` is neither a pull request's number nor its address",
                    review.pull_request
                ),
            )));
        };
        // Before the Job exists, so a Session nobody has heard of leaves
        // nothing behind to explain.
        let holder = match &review.session_id {
            None => None,
            Some(id) => {
                let kept = self
                    .store()
                    .lock()
                    .await
                    .session(id.as_str())
                    .map_err(|why| self.refusal(crate::adrift::Adrift::Writing(why)))?;
                let Some(kept) = kept else {
                    return Err(Refusal::Unacceptable(self.pr_refusal(
                        SESSION_UNKNOWN,
                        format!("no session `{}` has reported to Fleet", id.as_str()),
                    )));
                };
                Some(kept.harness)
            }
        };
        let (number, address) = match asked {
            Named::Address { number, address } => (number, address),
            Named::Number(number) => {
                let read = self.pull_request_read(&manifest_id, served.root(), number)?;
                (number, read.address)
            }
        };
        let proposal = ipc::ProposeJob {
            title: format!("Review pull request {number}"),
            // **Named here.** The proposer's reading of a link is the guess this
            // act exists to remove.
            workflow_id: ipc::WorkflowId::carried(CODE_REVIEW),
            owner_manifest_id: manifest_id,
            origin: core_model::TopLevelOrigin::SessionDispatched.into(),
            urgency: ipc::Urgency::from_wire("normal").expect("`normal` is an urgency"),
            atomic: false,
            model: None,
            acceptance_criteria: Vec::new(),
            subject: Some(ipc::Subject {
                kind: "pull_request".to_string(),
                reference: address.clone(),
            }),
            facts: String::new(),
            write_targets: None,
            dependencies: Vec::new(),
            attachments: Vec::new(),
            continue_from: None,
        };
        let job = self
            .propose_as(proposal, by)
            .await
            .map_err(|why| self.refusal(why))?;
        if let (Some(id), Some(harness)) = (&review.session_id, holder) {
            let origin = format!("dispatched from Session {}", id.as_str());
            let _ = self
                .report_session(SessionReport {
                    harness,
                    session_id: id.clone(),
                    fact: SessionFact::Attached {
                        attachment: AttachmentReport {
                            kind: "job".to_string(),
                            target: job.id().as_str().to_string(),
                            detail: BTreeMap::from([("origin".to_string(), origin)]),
                        },
                    },
                })
                .await;
        }
        Ok(ReviewDispatched {
            job_id: ipc::JobId::from(job.id()),
            address,
            session_id: review.session_id,
        })
    }
}

#[cfg(test)]
mod named_as {
    use super::{named, Named};

    #[test]
    fn a_number_is_a_number() {
        assert!(matches!(named(" 1843 "), Some(Named::Number(1843))));
    }

    #[test]
    fn an_address_ending_in_pull_and_a_number_is_one() {
        let said = "https://github.com/Mele-Labs/armada/pull/1843/";
        match named(said) {
            Some(Named::Address { number, address }) => {
                assert_eq!(number, 1843);
                assert_eq!(address, "https://github.com/Mele-Labs/armada/pull/1843");
            }
            _ => panic!("an address"),
        }
    }

    #[test]
    fn anything_else_is_neither() {
        for said in ["", "#1843", "armada", "https://github.com/Mele-Labs/armada/issues/12", "pull/12"] {
            assert!(named(said).is_none(), "{said}");
        }
    }
}

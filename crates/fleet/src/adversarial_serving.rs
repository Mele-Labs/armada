//! Fleet's side of the adversarial review's three operations: the one read-only call over the checkout
//! Fleet serves, the review kept for the plan version it read, and the person's two acts on a finding.
//! `docs/concepts/adversarial-review.md`; what the call is shown and how its answer is read is
//! `adversarial_review`.
//!
//! **No bay**, unlike an agent run about a pull request: a plan is read against the repository as it
//! stands, which is the served checkout, so nothing is leased and nothing is given back. Read-only by
//! construction all the same: `Reading::checkout` gives the call the three read tools and no other.
//!
//! **Only a press starts one.** A start answers at once with the review `running`; the call goes on a
//! task of its own, and the Job's `session.changed` is published when it starts, when it ends and when a
//! finding is settled.

use std::num::NonZeroU8;
use std::path::Path;
use std::sync::Arc;

use adapter_traits::{AgentHarness, Ask, Delivery, Reading, Vcs, WorkProduct};
use api::{AdversarialReviews, Refusal};
use ipc::{
    AdversarialReview, DismissFinding, FindingHow, ManifestId, ReviewSubject, SendFinding, SendSessionMessage,
    SessionId, SessionRow, SessionVoice, StartAdversarialReview, WireError,
};
use store::{KeptAdversarialFinding, KeptAdversarialReview, KeptPlan, Store};

use crate::adversarial_keeping::{how_word, kept_of, message, review_of, DIED, DONE, FAILED, RUNNING};
use crate::adversarial_review::{brief, findings_read, Shown};
use crate::daemon::Fleet;
use crate::review_runs::asked_and_costed;

/// No Job has the subject's id, the Job has no plan, or the plan never reached that version. A 404.
const REVIEW_SUBJECT_UNKNOWN: &str = "fleet.review_subject_unknown";
/// A start for a version the plan has been recorded past. A 409.
const REVIEW_SUBJECT_STALE: &str = "fleet.review_subject_stale";
/// No finding of that id in the review kept for the version. A 404.
const FINDING_UNKNOWN: &str = "fleet.finding_unknown";

/// Turns the call may take, its reads and its answer together: a pull request run's forty, since a
/// plan is checked against as many files as a change is.
const READ_TURNS: NonZeroU8 = NonZeroU8::new(40).expect("forty is not zero");

/// What marks a review going in this process, in the set an agent run's keys share.
fn going_key(id: &str, version: u32) -> String {
    format!("adversarial:{id}:{version}")
}

fn subject_of(id: &str, version: u32) -> ReviewSubject {
    ReviewSubject::Plan { session_id: SessionId::carried(id), version }
}

/// The subject's Job and version, as the store keys them.
fn named(subject: &ReviewSubject) -> (String, u32) {
    let ReviewSubject::Plan { session_id, version } = subject;
    (session_id.as_str().trim().to_string(), *version)
}

/// What the call answered, or why it did not, and what it cost either way.
type Read = Result<(Vec<KeptAdversarialFinding>, Option<u64>), (String, Option<u64>)>;

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
    /// `start_adversarial_review`. A review kept for the version is answered as it stands unless the
    /// press said again, and one still running is answered whatever it said.
    pub(crate) async fn adversarial_review_started(self: Arc<Self>, start: StartAdversarialReview) -> Result<AdversarialReview, Refusal> {
        let (id, version) = named(&start.subject);
        let kept = {
            let store = self.store().lock().await;
            if self.plan_held(&store, &id)?.version != version {
                return Err(self.refused(Refusal::IllegalMove, REVIEW_SUBJECT_STALE, "the plan was recorded again since"));
            }
            store.adversarial_review(&id, version).map_err(|why| self.ledger_fault(why))?
        };
        let key = going_key(&id, version);
        if let Some(mut kept) = kept {
            if kept.state == RUNNING {
                if self.review_going().lock().await.contains(&key) {
                    return Ok(review_of(&start.subject, &kept));
                }
                kept = self.died(kept);
                self.adversarial_kept(&id, &kept).await;
            }
            if !start.again {
                return Ok(review_of(&start.subject, &kept));
            }
        }
        if !self.review_going().lock().await.insert(key.clone()) {
            // Another press started it between the read and here.
            let kept = self.store().lock().await.adversarial_review(&id, version).ok().flatten();
            if let Some(kept) = kept {
                return Ok(review_of(&start.subject, &kept));
            }
        }
        let running = KeptAdversarialReview {
            version,
            state: RUNNING.to_string(),
            started_at: self.now().as_str().to_string(),
            finished_at: None,
            failed: None,
            cost_micros: None,
            findings: Vec::new(),
        };
        self.adversarial_kept(&id, &running).await;
        let answer = review_of(&start.subject, &running);
        tokio::spawn(async move { self.adversarial_review_driven(id, running, key).await });
        Ok(answer)
    }

    /// `send_finding`: the finding goes to the agent as the person's message, then it is settled. **Sent
    /// before it is settled**, so a send that fails leaves it waiting.
    pub(crate) async fn finding_sent(self: Arc<Self>, send: SendFinding) -> Result<AdversarialReview, Refusal> {
        let (id, kept, at) = self.finding_held(&send.subject, &send.finding_id).await?;
        let finding = &kept.findings[at];
        let item = format!("adversarial:{}:{}", kept.version, finding.id);
        let settling = self.hosts().settling(&id, &item);
        if finding.settled.is_some() || settling.is_none() {
            return Ok(review_of(&send.subject, &kept));
        }
        let sent = SendSessionMessage {
            session_id: SessionId::carried(&id),
            text: message(finding),
            attachments: Vec::new(),
            mentions: Vec::new(),
        };
        Arc::clone(&self).send_known(None, sent).await?;
        self.finding_settled(&send.subject, &kept, at, FindingHow::Sent).await
    }

    /// `dismiss_finding`: the finding is settled and the agent is told nothing.
    pub(crate) async fn finding_dismissed(self: Arc<Self>, dismiss: DismissFinding) -> Result<AdversarialReview, Refusal> {
        let (id, kept, at) = self.finding_held(&dismiss.subject, &dismiss.finding_id).await?;
        let item = format!("adversarial:{}:{}", kept.version, kept.findings[at].id);
        let settling = self.hosts().settling(&id, &item);
        if kept.findings[at].settled.is_some() || settling.is_none() {
            return Ok(review_of(&dismiss.subject, &kept));
        }
        self.finding_settled(&dismiss.subject, &kept, at, FindingHow::Dismissed).await
    }

    /// The review on the Job's record. One the store says is running that no task of this process holds
    /// died with the Fleet that ran it, and reads as failed until a press keeps it so.
    pub(crate) fn adversarial_on_record(&self, id: &str, kept: &KeptAdversarialReview) -> AdversarialReview {
        let subject = subject_of(id, kept.version);
        // A set held this instant is a review being started or ended, so it counts as going.
        let going = || self.review_going().try_lock().map_or(true, |going| going.contains(&going_key(id, kept.version)));
        match kept.state == RUNNING && !going() {
            true => review_of(&subject, &self.died(kept.clone())),
            false => review_of(&subject, kept),
        }
    }

    /// The call, from the brief to the kept answer. **Whatever happens the review ends**, so nothing is
    /// left running.
    async fn adversarial_review_driven(self: Arc<Self>, id: String, mut review: KeptAdversarialReview, key: String) {
        match self.adversarial_review_read(&id, review.version).await {
            Ok((findings, cost)) => {
                review.state = DONE.to_string();
                review.findings = findings;
                review.cost_micros = cost;
            }
            Err((said, cost)) => {
                review.state = FAILED.to_string();
                review.failed = Some(said);
                review.cost_micros = cost;
            }
        }
        review.finished_at = Some(self.now().as_str().to_string());
        self.adversarial_kept(&id, &review).await;
        self.review_going().lock().await.remove(&key);
    }

    /// One call on `models.review`, shown the person's words, the plan's and the repository's rules.
    async fn adversarial_review_read(&self, id: &str, version: u32) -> Read {
        let spent = |said: &str| (said.to_string(), None);
        let reviewing = self.reviewing().map_err(|why| (format!("no agent could be asked: {why:?}"), None))?;
        let (session, plan) = {
            let store = self.store().lock().await;
            (store.session(id).ok().flatten(), store.session_plan(id).ok().flatten())
        };
        let plan = plan.filter(|plan| plan.version == version).ok_or_else(|| spent("the plan was recorded again before it was read"))?;
        let manifest = session.and_then(|session| session.manifest_id).ok_or_else(|| spent("the job is in no repository Fleet serves"))?;
        let served = self.served_named(Some(&ManifestId::carried(&manifest))).map_err(|_| spent("the repository is not served"))?;
        let rows = self.rows_of(id).await.map_err(|_| spent("the thread would not read"))?;
        let mut his = rows.into_iter().filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => Some(text),
            _ => None,
        });
        let request = his.next().unwrap_or_default();
        let later: Vec<String> = his.collect();
        let plan = crate::plan_record::wire(&plan);
        let cap = self.tuned(|dials| dials.standing_rules_bytes);
        let standing = crate::judging::standing(served.manifest(), served.root(), cap).told(&self.prompts().wording());
        let asked = brief(&Shown { request: &request, later: &later, plan: &plan, standing: &standing });

        let ask = Ask::put(reviewing.model.clone(), &asked, reviewing.environment.clone())
            .and_then(|ask| Ok(ask.reading(Reading::checkout(served.root(), READ_TURNS)?)))
            .map_err(|why| (format!("the question could not be put: {why:?}"), None))?;
        let (answered, micros) = asked_and_costed(&reviewing, &ask).await;
        let answer = answered.map_err(|why| {
            // The full output is Fleet's to keep, and the card gets one line of it.
            eprintln!("adversarial review of job {id}, version {version}: the agent did not answer: {why}");
            (why.in_a_line("review"), micros)
        })?;
        let root = served.root().to_string();
        let file = move |path: &str| std::fs::read_to_string(Path::new(&root).join(path)).ok();
        let findings = findings_read(&answer, &asked, &plan, &file).map_err(|why| (format!("the answer was refused: {why}"), micros))?;
        Ok((findings.iter().map(|one| kept_of(one, &plan)).collect(), micros))
    }

    /// Keep the review and publish the Job. **A store that will not take it does not stop the review**:
    /// the Job is published from what the store holds, and a press reads it again.
    async fn adversarial_kept(&self, id: &str, kept: &KeptAdversarialReview) {
        let _ = self.store().lock().await.keep_adversarial_review(id, kept);
        let _ = self.published_hosted(id).await;
    }

    fn died(&self, mut kept: KeptAdversarialReview) -> KeptAdversarialReview {
        kept.state = FAILED.to_string();
        kept.failed = Some(DIED.to_string());
        kept.finished_at = Some(self.now().as_str().to_string());
        kept
    }

    /// The Job's plan, or the subject is unknown.
    fn plan_held(&self, store: &Store, id: &str) -> Result<KeptPlan, Refusal> {
        let unknown = || self.refused(Refusal::NotFound, REVIEW_SUBJECT_UNKNOWN, "no such Job, or it has no plan");
        store.session(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(unknown)?;
        store.session_plan(id).map_err(|why| self.ledger_fault(why))?.ok_or_else(unknown)
    }

    /// The review kept for the subject's version, and where in it the finding is. **A version the plan was
    /// recorded past still answers**: a finding on an out-of-date review is sent or dismissed all the same.
    async fn finding_held(&self, subject: &ReviewSubject, finding_id: &str) -> Result<(String, KeptAdversarialReview, usize), Refusal> {
        let (id, version) = named(subject);
        let store = self.store().lock().await;
        if version > self.plan_held(&store, &id)?.version {
            return Err(self.refused(Refusal::NotFound, REVIEW_SUBJECT_UNKNOWN, "the plan never reached that version"));
        }
        let unknown = || self.refused(Refusal::NotFound, FINDING_UNKNOWN, &format!("no finding {finding_id} in this review"));
        let kept = store.adversarial_review(&id, version).map_err(|why| self.ledger_fault(why))?.ok_or_else(unknown)?;
        let at = kept.findings.iter().position(|one| one.id == finding_id).ok_or_else(unknown)?;
        Ok((id, kept, at))
    }

    async fn finding_settled(
        &self,
        subject: &ReviewSubject,
        kept: &KeptAdversarialReview,
        at: usize,
        how: FindingHow,
    ) -> Result<AdversarialReview, Refusal> {
        let (id, version) = named(subject);
        let now = self.now().as_str().to_string();
        let settled = {
            let mut store = self.store().lock().await;
            store
                .settle_adversarial_finding(&id, version, &kept.findings[at].id, how_word(how), &now)
                .map_err(|why| self.ledger_fault(why))?;
            store.adversarial_review(&id, version).map_err(|why| self.ledger_fault(why))?
        };
        let _ = self.published_hosted(&id).await;
        Ok(review_of(subject, settled.as_ref().unwrap_or(kept)))
    }

    fn refused(&self, how: fn(WireError) -> Refusal, code: &'static str, said: &str) -> Refusal {
        how(WireError::raised(code, said, self.run_id()))
    }
}

impl<H, V, W> AdversarialReviews for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn start_adversarial_review(self: Arc<Self>, start: StartAdversarialReview) -> Result<AdversarialReview, Refusal> {
        self.adversarial_review_started(start).await
    }

    async fn send_finding(self: Arc<Self>, send: SendFinding) -> Result<AdversarialReview, Refusal> {
        self.finding_sent(send).await
    }

    async fn dismiss_finding(self: Arc<Self>, dismiss: DismissFinding) -> Result<AdversarialReview, Refusal> {
        self.finding_dismissed(dismiss).await
    }
}

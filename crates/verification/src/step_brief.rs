//! A Judge's brief about one step of a Job, which is the half of [`Brief`] that
//! needs a Job.
//!
//! **Beside `judge` rather than in it** so that the brief, its reading and its
//! words carry no Job: a Session's judge builds the same [`Brief`] from
//! [`Work`](crate::Work) and reads it back with the same [`Brief::read`], and
//! only the step, the request and the product below need a Job to exist.

use config::ResolvedStep;
use core_model::JudgeCriterion;

use crate::answered::Answered;
use crate::judge::{Brief, JUDGE_ANSWER, JUDGE_OPENING, JUDGE_QUESTION};
use crate::product::{Product, Reference};
use crate::request::Request;
use crate::shown::Laid;
use crate::standing::Standing;
use crate::wording::Wording;

/// What the request's part of a brief is called on a citation.
const REQUEST: &str = "request";

/// What the repository's standing rules are called on a citation.
const STANDING: &str = "standing_rules";

impl Brief {
    /// Assemble the question for one criterion.
    ///
    /// There is no `transcript` parameter and no `submission` parameter. What
    /// the Drone said about how its step went is not an input to the thing
    /// checking it — and what the step *produced* arrives as a [`Product`],
    /// which is a different object with a different rule.
    ///
    /// `request` is not optional, and it is the whole of what #169 closed. A
    /// criterion that compares the work against what was asked for — *does this
    /// scope note address what was actually requested*, *does this plan address
    /// what was actually asked* — is answerable here because every call carries
    /// it, and there is no arrangement of arguments that assembles a brief
    /// without it. `standing` sits after it; [`Standing::unstated`] adds nothing.
    pub fn about(
        step: &ResolvedStep,
        criterion: &JudgeCriterion,
        request: Request<'_>,
        standing: &Standing,
        product: &Product<'_>,
        references: &[Reference<'_>],
        answered: Answered<'_>,
    ) -> Brief {
        Brief::worded(
            step,
            criterion,
            request,
            standing,
            product,
            references,
            answered,
            &Wording::shipped(),
        )
    }

    /// The same brief in `wording`: the pieces a caller replaced, every other
    /// as it ships. Material is laid exactly where [`Brief::about`] lays it.
    #[allow(clippy::too_many_arguments)]
    pub fn worded(
        step: &ResolvedStep,
        criterion: &JudgeCriterion,
        request: Request<'_>,
        standing: &Standing,
        product: &Product<'_>,
        references: &[Reference<'_>],
        answered: Answered<'_>,
        wording: &Wording,
    ) -> Brief {
        // **Laid in labelled parts, and the text is byte for byte what it
        // was.** The labels are what a citation says it landed in, and they
        // are recorded at assembly rather than recognised afterwards — see
        // [`shown`](mod@crate::shown).
        let mut laid = Laid::new();
        laid.loose(wording.get(JUDGE_OPENING));
        laid.loose("\n\n");
        laid.loose(&format!("Step: {}\n\n", step.label()));
        // **First, and above the step's own evidence.** The request is the
        // outermost yardstick — an earlier step's note is measured against it
        // too — so it is read before anything it is the standard for, which is
        // the ordering `Reference::parts` already argues for one level down.
        laid.part(REQUEST, &request.told(wording));
        // Beside the request and for its reason: a standard, not the work.
        laid.part(STANDING, &standing.told(wording));
        // **The deterministic facts, and the whole of them.** A name and an
        // outcome word is what this rendered until #205, which dropped the two
        // parts a criterion is actually answered from: why a Check that did not
        // run did not, and what a Check that ran observed. A Judge asked
        // whether a suite covers a case was answering off the diff while the
        // suite's own output sat unread.
        for (label, part) in answered.parts(wording) {
            laid.part(&label, &part);
        }
        // The yardstick before the product, the way the gaming brief puts its
        // baseline first: what the work is measured against is context for
        // reading it, and it is labelled as not being the thing under judgment.
        for (label, part) in Reference::parts(references, wording) {
            laid.part(&label, &part);
        }
        for (label, part) in product.parts(wording) {
            laid.part(&label, &part);
        }
        laid.loose("\n");
        laid.loose(wording.get(JUDGE_QUESTION));
        laid.loose("\n\n");
        laid.loose(&criterion.question);
        laid.loose("\n\n");
        laid.loose(wording.get(JUDGE_ANSWER));
        let (question, regions) = laid.done();
        Brief {
            criterion: criterion.criterion_id.clone(),
            question,
            regions,
            asks_risk: false,
        }
    }
}

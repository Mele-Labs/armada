//! A reference to what an earlier step produced.

use alloc::string::String;

use crate::workflow::ids::StepId;


/// A reference to what an earlier step produced, as `baseline_ref` spells it:
/// `<step_id>.evidence`.
///
/// **There is no constructor taking a bare [`StepId`].** The suffix is the
/// registry's own form for an evidence reference, so a value that did not go
/// through [`EvidenceRef::parse`] cannot exist, and `reference_docs` and
/// `baseline_ref` cannot drift into two spellings of one thing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRef {
    step: StepId,
}

/// The suffix an evidence reference ends in.
const EVIDENCE_SUFFIX: &str = ".evidence";

impl EvidenceRef {
    /// Read `<step_id>.evidence`. `None` for anything else, including a bare
    /// step id — naming a step is not naming its evidence.
    pub fn parse(value: &str) -> Option<EvidenceRef> {
        let step = value.strip_suffix(EVIDENCE_SUFFIX)?;
        match step.is_empty() {
            true => None,
            false => Some(EvidenceRef {
                step: StepId::new(step),
            }),
        }
    }

    /// Which step's evidence. **Not enough on its own to reach it** — see
    /// `fleet`'s `AtStep::baseline`, which will not answer with a step that is
    /// not strictly earlier.
    pub fn step(&self) -> &StepId {
        &self.step
    }

    /// The reference as it was written.
    pub fn as_wire(&self) -> String {
        let mut wire = String::from(self.step.as_str());
        wire.push_str(EVIDENCE_SUFFIX);
        wire
    }
}

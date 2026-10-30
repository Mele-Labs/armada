//! What a rule does when an act fails: tell the agent or ask the owner, after a capped number of
//! retries.

use core::fmt;

/// How many times a failed act is tried again before the rule gives up.
///
/// **Capped at [`Retries::CAP`]**, so a rule file cannot write an act that tries for ever against a
/// tracker that keeps refusing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Retries(u8);

impl Retries {
    /// The most a rule may ask for.
    pub const CAP: u8 = 5;

    pub fn new(times: u32) -> Result<Retries, OverTheCap> {
        match u8::try_from(times).ok().filter(|times| *times <= Retries::CAP) {
            Some(times) => Ok(Retries(times)),
            None => Err(OverTheCap { asked: times }),
        }
    }

    pub fn times(self) -> u8 {
        self.0
    }
}

/// More retries than the cap lets a rule ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverTheCap {
    pub asked: u32,
}

impl fmt::Display for OverTheCap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} retries is more than the {} a rule may ask for", self.asked, Retries::CAP)
    }
}

impl core::error::Error for OverTheCap {}

/// Whom a failure that retrying did not mend goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Then {
    /// Tell the Job's agent, which is in the Job and can look.
    TellAgent,
    /// Put it in the owner's Needs you.
    AskOwner,
}

impl Then {
    pub const ALL: &'static [Then] = &[Then::TellAgent, Then::AskOwner];

    pub fn as_wire(self) -> &'static str {
        match self {
            Then::TellAgent => "tell_agent",
            Then::AskOwner => "ask_owner",
        }
    }

    pub fn from_wire(value: &str) -> Option<Then> {
        Then::ALL.iter().copied().find(|then| then.as_wire() == value)
    }
}

/// How a rule's failure is handled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OnFailure {
    pub retries: Retries,
    pub then: Then,
}

impl Default for OnFailure {
    /// **Two retries, then ask the owner**: a write that failed three times is not one an agent
    /// sees more of than the owner would, and a rule that says nothing should not send an agent
    /// round a loop.
    fn default() -> OnFailure {
        OnFailure { retries: Retries(2), then: Then::AskOwner }
    }
}

# Where does a person wait for the Job proposer?

**Decided 2026-09-30.**

He said, against `DispatchRequest.tsx:856`: *"I thought when the job is being
read by the proposer it was going to be a job. We should be in the
arc/proposing-review state. So basically once I dispatch a job and the proposer
is running, it is a job and I should be taken to the job page with the details
being filled in."*

**It cannot be a Job, and he was told why before he chose.** One dispatch can
become several: `propose_from_request` in `crates/ipc/operations.toml` answers
with every Job the request became, all at `awaiting_approval`, and the proposer
does not know how many until it finishes. `crates/ipc/src/proposing.rs` states
the same from the other side — a proposal is the interval before any Job exists,
so it hangs off nothing and needs a `ProposalId` of its own. A Job invented for
the wait would have to be unmade or renumbered when the answer came back with
two.

**Chosen:** make the waiting screen a destination. Dispatch leaves the composer
immediately and lands on a page of its own for the proposal — the request, the
model reading it, the elapsed against the budget, how far the call has got, and
Stop. When the proposer answers the screen becomes the Job it drafted, or a
short list where it drafted several. No Job is invented, and nobody waits inside
a form.

**Cost he took:** a new destination to design, and a second screen on the way to
a Job.

**What that moved, beyond the new screen:**

- The wait, the proposal's answer and both refusals left `DispatchRequest`,
  which is a form again — one field, what is attached, Settings and one press.
- A stop is drawn for the whole of the wait rather than appearing at
  `PROPOSAL_IS_SLOW`. The mark still adds the sentence; what changed is that a
  screen a person was sent to in order to watch one call carries the only act on
  it from the first frame.
- `fleet.proposer_stopped` is read rather than falling through to a bare
  refusal. `crates/fleet/src/refusing.rs` has said since it landed that the code
  exists so a client does not draw a person's own press as Armada breaking, and
  nothing on Bridge's side was matching it — so a stop drew a red failure notice
  on another surface.
- A window reopened while its own proposer is out adopts the call and lands on
  this page. Leaving the page is leaving it: the rail works, and the same call
  is not adopted twice.

**Not decided here:** what a window that abandoned a proposal shows about it.
Fleet keeps publishing `proposal.moved` and the Jobs land on the board when it
answers, so nothing is lost — but nothing signposts the call either.

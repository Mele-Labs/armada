// One proposal, while the Job proposer is still reading it. Mirrors
// `crates/ipc/src/proposing.rs`.
//
// **Its own file rather than another block in `protocol.ts`**, which is where
// it started and which the 900-line refusal ended. The split is the one the
// Rust side already draws: these types are about the interval before a Job
// exists, and every other DTO in that file is about a Job that does.

/**
 * One Job proposer call, while it is still out. `crates/ipc/src/proposing.rs`.
 *
 * **`JudgeInFlight` one step earlier, and the step is the difference.** That one
 * rides on a step of a Job and arrives on the open Job's detail; a proposal is
 * the interval before any Job exists, so it arrives only as `proposal.moved` and
 * there is nothing to read it off.
 *
 * **This one ticks.** A person is waiting in front of a form, and an elapsed
 * count draws "thinking hard" and "never reached the vendor" identically.
 */
export type ProposalInFlight = {
  /** What a stop names. Fleet minted it; a client learns it from here. */
  proposal_id: string;
  /**
   * The caller's own token, echoed unchanged. **How a client recognises its own
   * call** — `proposal_id` is minted after the request arrives, so there is
   * nothing else to match on, and matching the request's text would match the
   * wrong call when two people dispatch the same words.
   */
  client_ref?: string;
  /** Which model is reading the request. What the wait costs, roughly. */
  model: string;
  /** When the call went out. A surface subtracts for itself. */
  since: string;
  /**
   * How long Fleet will wait before giving up. **What makes the elapsed count
   * mean something** — against nothing it can only say "slow"; against the
   * ceiling it can say how much of the decision is left.
   */
  budget_ms: number;
  /** How far the call has got. */
  reached: ProposalReach;
  /**
   * The harness's own running estimate of how much the model has thought.
   * Cumulative within this call and **an estimate** — drawn as an approximation,
   * never as a billed figure. Absent before it starts, and on a model that does
   * not think.
   */
  thinking_tokens?: number;
  /**
   * How much of the answer has arrived, in characters. **A count and never the
   * text**, which is unchanged — the raw answer does not cross this seam.
   *
   * It used to be the whole of what a surface could say about the answer, on
   * the grounds that a channel carrying it as it was written would be a second,
   * earlier, worse copy of the Jobs it minted. True while nothing existed until
   * the answer landed; a dispatched request is a Job from the press since 30 Sep
   * 2026, so `settled` is that row becoming more complete rather than a rival to
   * it. The Rust doc carries the correction in full.
   */
  answered_characters?: number;
  /**
   * What the proposer has decided so far. Absent until something has, and on
   * every Fleet older than 19.1.
   */
  settled?: ProposalSettled;
};

/**
 * What the proposer has decided, while it is still writing the rest.
 * `crates/ipc/src/proposing.rs`.
 *
 * **Four fields in the owner's order** (30 Sep 2026): the workflow decides the
 * Job's shape, the title is what makes the row recognisable, done-when is the
 * goal, and the settings are the part he can still change.
 *
 * **Fields that are settled, never a transcript.** A field appears once its own
 * line in the answer has ended, so a client either has a title or has none and
 * never has half of one. No `scope`, no `because`, no `after`: each of those
 * reaches a client as the Job it belongs to, once the call has answered.
 */
export type ProposalSettled = {
  /** The workflow it chose. **Only ever one this repository holds.** */
  workflow_id?: string;
  /**
   * What the Job is called. **The field that changes a row under a reader** —
   * before it lands the row's title is the request as it was typed.
   */
  title?: string;
  /** What the Job is held to, one line each, in the order they arrived. */
  done_when?: string[];
  /** The settings it decided. Absent until the line has ended. */
  settings?: ProposalSettings;
};

/**
 * The settings the proposer answered. `crates/ipc/src/proposing.rs`.
 *
 * Both arrive on one line of the answer, so both settle together — the settings
 * are one field, which is why they are a type rather than two fields on
 * `ProposalSettled`.
 *
 * **Land-as-one is not here and that is a decision.** How the work lands follows
 * from having read the code and this call has read none — the 3 Sep 2026 ruling,
 * kept on 30 Sep when it was put beside the model.
 */
export type ProposalSettings = {
  /** How urgent it read the request as being — the generated urgency vocabulary. */
  urgency?: string;
  /**
   * Which model a Drone on this Job will be spawned as. **Only ever one this
   * machine holds**, and absent is configuration deciding rather than a model
   * the proposer picked as a default.
   */
  model?: string;
};

/**
 * How far a proposal has got. `crates/ipc/src/proposing.rs`.
 *
 * **A union here where `JudgeInFlight.look` is a bare `string`**, and the
 * difference is who decides. A look is decided by the call sites that make one,
 * so a roster here would have no authority behind it. These five are decided by
 * the shape of a model call, and a surface draws a different sentence for each.
 */
export type ProposalReach =
  /**
   * Started and has said nothing yet. **The one worth telling apart**: ninety
   * seconds here never reached the vendor at all, which will not resolve by
   * waiting — the opposite decision from every other value.
   */
  | "starting"
  /** The harness is up. It has not asked yet. */
  | "started"
  /** The question is at the vendor. Everything after this is the model's time. */
  | "requesting"
  /** Thinking. `thinking_tokens` says how much. */
  | "thinking"
  /**
   * The answer is arriving. `answered_characters` says how much. **Nearly
   * over** — stopping here throws away work about to land.
   */
  | "answering";

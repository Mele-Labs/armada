// One call a hosted Job makes that the person's own settings do not cover, held open while they
// answer it. `crates/ipc/src/call.rs`, #1389.
//
// **A `CommandInFlight` one subject over, and deliberately not that type.**
// Every field of that one is about a job — `step_id`, `allow_for_job`, a rule
// written into `armada.yml` on the job's branch — and such a call has no job.
// The two sets of answers are different sets, and a shared union would offer a
// person an answer fleet cannot take.
//
// The header rules in `protocol.ts` hold here: these closed sets are the seam's own, with no
// registry behind them.

/**
 * What a person may answer about one call. Since protocol 14.18.
 *
 * `allow_once` runs it and writes nothing. `allow_and_remember` runs it and
 * writes `rule` into the repository's own personal agent settings, so the agent
 * CLI allows it without asking again — for the call and in their terminal alike.
 * `refuse` tells the session no.
 *
 * `always_allow` and `always_block` are offered on a card over a session's reach past its
 * slots, and only where `reaches` names the path. Each carries `folder`, the path or one above
 * it, which fleet puts on the folder list (`features.reachFolders`) every session's guard reads.
 * **Never a CLI rule**: the rule matches a command, and the guard runs before the CLI's settings.
 *
 * `allow_for_session` carries `folder` the same way and lets only the session that asked reach it,
 * until that session is closed. Fleet holds it in memory: nothing goes on `features.reachFolders`,
 * and a folder the owner blocked stays blocked.
 *
 * `allow_like_this_for_session` carries `kind`, the card's own or the same with no script, and lets
 * the session's calls of that kind run with no card until it is closed, narrowed to `folder` where
 * one is given. Held in Fleet's memory the same way; a removal or a push is still read as one.
 *
 * **Offered, never assumed.** `offers` carries the subset fleet will take, and
 * an answer outside it is a 409.
 *
 * **`allow_every_read` was here in 17.3 and is gone in 18.0.** It was offered
 * on a read of armada's own door; #1525 made a door read run without anybody
 * being asked, so no card is drawn over one and the answer had nothing to
 * appear on.
 */
/** Every answer, for a process that checks one before sending it on. */
export const CALL_ANSWERS = [
  "allow_once",
  "allow_and_remember",
  "refuse",
  "always_allow",
  "allow_for_session",
  "allow_like_this_for_session",
  "always_block",
] as const;
export type CallAnswer = (typeof CALL_ANSWERS)[number];

/** Whether a call past a session's slots reads what it names, writes to it, or runs something the guard cannot class as either. */
export type ReachesTo = "read" | "write" | "run";

/** The kind of a shell call: its program, and for an interpreter the script it runs. */
export type CallKind = {
  program: string;
  script?: string;
};

/**
 * One call waiting on a person to answer, right now. Since
 * protocol 14.18.
 *
 * **Not a status, and no job.** Nothing is queued and no step is running while
 * it waits — what is waiting is one process, inside one tool call, for the
 * length of one reply. `manifest_id` is what names where it came from.
 */
export type CallInFlight = {
  /** Fleet's id for this ask, and what an answer names. */
  call: string;
  manifest_id: string;
  asked_at: string;
  /** The tool reached for, in the harness's own spelling. */
  tool: string;
  /** The command or the argument, one line. Empty is a tool with no named argument. */
  detail: string;
  truncated: boolean;
  length?: number;
  /** The argument as the session sent it, set only where `detail` was cut: the person allows all of it. */
  whole?: string;
  /** The rule that would have to allow it, in the CLI's settings spelling — `Bash(gh issue list:*)`. */
  rule: string;
  /** In the order to draw them. */
  offers: CallAnswer[];
  /** How long fleet holds the call open, from `asked_at`. Fleet's bound, said rather than derived. */
  holding_for_seconds: number;
  /** The agent's questions, where the call is it asking the person. Absent for every other call. Since protocol 23.71. */
  questions?: AskedQuestion[];
  /** The absolute path past a session's slots the guard asked about. The folder an "always" answer names is this or one above it. */
  reaches?: string;
  /** Whether the call reads or writes past the slots, on a card the guard drew over a reach. Set where `reaches` is, and where the path could not be read as one. */
  reaches_to?: ReachesTo;
  /** The program and script of the Bash segment whose reach the card is about, and what `allow_like_this_for_session` matches. Set only on a Bash reach card. */
  kind?: CallKind;
};

/** One question the agent put to the person. Since protocol 23.71. */
export type AskedQuestion = {
  /** Also the key its answer goes under. */
  question: string;
  header: string;
  multi_select: boolean;
  options: AskedChoice[];
};

export type AskedChoice = {
  label: string;
  description: string;
  /** Monospace text the agent attached to show while the option is in focus. Absent where it sent none. */
  preview?: string;
};

/** What a person chose for one question: option labels, and their own words where they chose Other. Since protocol 23.71. */
export type QuestionAnswer = {
  question: string;
  chosen: string[];
};

/** The four option labels of the question a Job asked about carrying on in a new Job before Fleet raised its own `handoff` card, whose options carry their labels. Off the wire; Classic's mock and walks still draw the old card. */
export const HANDOFF_YES = "Carry on in a new Job";
export const HANDOFF_STAY = "Stay here";
export const HANDOFF_QUIET = "Stay, don't ask again this Job";
export const HANDOFF_REMIND = "Remind me at a higher limit";
/** The question the old card put after `HANDOFF_REMIND`, and its figures in tokens, labelled `450k`. Off the wire, as above. */
export const HANDOFF_REMIND_QUESTION = "Ask again at";
export const HANDOFF_REMIND_AT = [
  450_000,
  500_000,
  600_000,
] as const;

// Autopilot on the wire, mirrored by hand from `crates/ipc/src/autopilot.rs`. #2092,
// `docs/concepts/autopilot.md`.
//
// The header rules in `protocol.ts` hold here.

/** Whether it picks. `stopped` is the end: a new start replaces it. */
export type AutopilotState = "running" | "paused" | "stopped";

/**
 * The cap that paused it. It stays named until a person's Resume, which Fleet refuses (409
 * `fleet.autopilot_capped`, naming the cap) while any cap is still reached. #2105.
 */
export type AutopilotCap = "until" | "spend" | "jobs";

/** What became of one pick. `withdrawn`: its Job had not started when Autopilot was stopped. */
export type AutopilotPickState = "started" | "set_aside" | "withdrawn" | "dismissed";

/** One pick. `workflow_id` is absent where the picker was unsure of that too. */
export type AutopilotPick = {
  pick_id: string;
  /** The issue's own name in its tracker: `#123`, `ABC-123`. */
  issue_reference: string;
  issue_title: string;
  issue_url: string;
  workflow_id?: string;
  job_id?: string;
  state: AutopilotPickState;
  /** The picker's reason, in its words. */
  reason: string;
  at: string;
};

/**
 * One repository's Autopilot, whole: `GET /autopilots/:repository`, every act's answer, and
 * `autopilot.changed`. `at_work` is how many of its Jobs hold a place now; a Job waiting on a
 * person holds none.
 */
export type Autopilot = {
  manifest_id: string;
  instruction: string;
  at_once: number;
  until?: string;
  spend_cap_micros?: number;
  job_cap?: number;
  state: AutopilotState;
  started_at: string;
  ended_at?: string;
  /** Where a cap rather than a person paused it. Cleared by Resume. */
  capped_by?: AutopilotCap;
  spent_micros: number;
  jobs_started: number;
  at_work: number;
  picks: AutopilotPick[];
  /** Why the last pass picked nothing, in Fleet's words. Absent once a pass goes through. */
  fault?: string;
};

/** `GET /autopilots`: every repository's, stopped ones included. */
export type AutopilotList = { autopilots: Autopilot[] };

/** `POST /autopilots/:repository/proposal`: the sentence, and the person's own clock with its offset. */
export type ProposeAutopilot = { sentence: string; local_now?: string };

/**
 * What one sentence was read as, for the person to confirm or change, and the body of
 * `POST /autopilots/:repository` and of `POST /autopilots/:repository/settings`, which changes a
 * running or paused one in its state. **Nothing starts from a proposal**: the start takes what they confirmed.
 */
export type AutopilotSettings = {
  instruction: string;
  at_once: number;
  until?: string;
  spend_cap_micros?: number;
  job_cap?: number;
};

/**
 * Per setting, the person's own words it was read from, keyed as `AutopilotSettings` keys them.
 * Absent where the sentence said nothing about it, and where the reader quoted words the sentence
 * does not hold. #2105.
 */
export type AutopilotQuoted = {
  instruction?: string;
  at_once?: string;
  until?: string;
  spend_cap_micros?: string;
  job_cap?: string;
};

/**
 * `POST /autopilots/:repository/proposal`'s answer: the settings, flattened in, and the words each
 * was read from. A start sends back the settings alone. #2105.
 */
export type AutopilotProposal = AutopilotSettings & { quoted: AutopilotQuoted };

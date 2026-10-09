// What the Phone Gateway sends the phone (crates/pocket/src/phone.rs, live.rs).
// Hand-mirrored: the Gateway trims Fleet's Job to an allowlist, so this is not
// `@armada/protocol`'s JobSummary and nothing here is generated.

export type PhoneStep = { at: number; of: number; name: string };

export type PhoneCheck = { name: string; passed: boolean };

export type PhoneJob = {
  id: string;
  title: string;
  repository?: string;
  /** The status as Fleet spells it, e.g. `escalated`, `awaiting_review`. */
  status: string;
  /** Why it stopped: the escalation trigger's name. */
  reason?: string;
  /** A Drone is waiting on an answer. */
  asking: boolean;
  step?: PhoneStep;
  /** RFC 3339 instants; the phone counts ages itself. */
  created_at: string;
  started_at?: string;
  ended_at?: string;
  waiting_since?: string;
  verdict?: "pass" | "veto";
  checks?: PhoneCheck[];
  pull_request?: string;
};

/** GET /api/needs */
export type NeedsBody = { needs: PhoneJob[] };
/** GET /api/jobs?state=running|done */
export type JobsBody = { jobs: PhoneJob[] };
/** GET /api/jobs/:id is a PhoneJob. */

/** One `data:` line of GET /api/live. `resync` means read the lists again. */
export type LiveChange = {
  change: "created" | "status" | "step" | "landed" | "forgotten" | "resync";
  job_id?: string;
  title?: string;
  status?: string;
  reason?: string;
};

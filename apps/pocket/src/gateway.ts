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
export type NeedsBody = { needs: PhoneJob[]; sessions: PhoneSession[] };
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

export type PhoneChoice = { label: string; description?: string };

export type PhoneQuestion = {
  question: string;
  header?: string;
  multi_select: boolean;
  options: PhoneChoice[];
};

/** What a hosted Session is held on. Never present on a Terminal Session. */
export type PhoneAsk = {
  ask_id: string;
  tool: string;
  detail?: string;
  /** Empty for a permission ask, which is answered with one of `offers`. */
  questions?: PhoneQuestion[];
  offers: ("allow_once" | "refuse")[];
};

export type PhoneSession = {
  id: string;
  title?: string;
  repository?: string;
  kind: "hosted" | "terminal";
  waiting: boolean;
  waiting_since?: string;
  ask?: PhoneAsk;
};

/** GET /api/sessions (live Sessions). */
export type SessionsBody = { sessions: PhoneSession[] };

/** One question's choice: the option labels, or the person's own words. */
export type ChosenAnswer = { question: string; chosen: string[] };

/**
 * POST /api/sessions/answer. 204 on success; 409 for a Terminal Session, one
 * not waiting, or a stale `ask_id`; the body is a sentence to show.
 * `answer` is a decision for a permission ask, the choices for a question.
 */
export type AnswerBody = {
  session_id: string;
  ask_id?: string;
  answer: "allow_once" | "refuse" | ChosenAnswer[];
};

/** GET /api/repositories: the labels `DispatchBody.repository` may be. */
export type RepositoriesBody = string[];

/** POST /api/jobs. 201 with the Jobs the text became; 400 for an unknown repository. */
export type DispatchBody = { text: string; repository: string };
export type DispatchedBody = { jobs: PhoneJob[] };

// What the Phone Gateway sends the phone (crates/pocket/src/phone_sessions.rs, live.rs).
// Hand-mirrored: the Gateway trims Fleet's Session to an allowlist, so this is not
// `@armada/protocol`'s SessionRecord and nothing here is generated.

/** One `data:` line of GET /api/live. `resync` means read the lists again. */
export type LiveChange = {
  change: "session" | "resync";
  session_id?: string;
};

export type PhoneChoice = {
  label: string;
  description?: string;
};

export type PhoneQuestion = {
  question: string;
  header?: string;
  multi_select: boolean;
  options: PhoneChoice[];
};

/** What a Session is held on. A Terminal Session has one only while Fleet holds its question. */
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
  /** A turn is running. */
  working: boolean;
  /** RFC 3339 instants; the phone counts ages itself. */
  started_at: string;
  ended_at?: string;
  waiting_since?: string;
  ask?: PhoneAsk;
  /** Held on its start card: a draft Job, which runs nothing until Send. */
  held: boolean;
  /** The Job that started it. */
  from?: {
    id: string;
    title: string;
  };
  context_tokens?: number;
  context_window?: number;
  /** The Job said, through all_done, that nothing is left. Present while the claim stands. */
  done_since?: string;
  done_summary?: string;
  /** What Fleet read of the turn. */
  standing?: "working" | "stuck" | "stopped" | "replied" | "done" | "idle";
  /** Parked by the owner: nothing is asked of him until he resumes it. */
  paused?: boolean;
  /** The plan's limit holds it until this instant. */
  limited_until?: string;
  /** What keeps it open besides the owner, in Fleet's order. */
  held_open_by?: PhoneHold[];
  /** It starts when another Job's condition is met. */
  starts_when?: {
    on: {
      id: string;
      title: string;
    };
    when: "ends" | "plan" | "pushed" | "lands";
    part_title?: string;
    pull_number?: number;
  };
  /** Why its agent would not start. */
  start_failed?: string;
  /** Not running and not parked: why, and what Fleet is doing about it. */
  stopped?: {
    reason: "turn_ended" | "tasks_open" | "process_gone" | "fleet_down";
    restart: "none" | "queued" | "running" | "failed";
    repair?: "running" | "failed" | "held";
    gave_up?: boolean;
    down?: {
      from: string;
      to: string;
    };
  };
  /** Its first open pull request whose Checks failed. */
  failing_pull?: number;
  /** Never spoken in: no title, no turn and nothing attached. */
  blank?: boolean;
  /** The proposer is reading the request, and it starts on the answer. */
  reading?: boolean;
};

export type PhoneHold =
  | {
      kind: "watch";
      name: string;
    }
  | {
      kind: "job";
      name: string;
    }
  | {
      kind: "pull_request";
      number: number;
      on: "queue" | "auto_merge" | "checks";
      auto_merge?: boolean;
    };

/** GET /api/sessions: the live Sessions only. The phone takes Needs you from the ones waiting. */
export type SessionsBody = {
  sessions: PhoneSession[];
};

/** One question's choice: the option labels, or the person's own words. */
export type ChosenAnswer = {
  question: string;
  chosen: string[];
};

/**
 * POST /api/sessions/answer. 204 on success; 409 for a Session not
 * holding a question, or a stale `ask_id`; the body is a sentence to show.
 * `answer` is a decision for a permission ask, the choices for a question.
 */
export type AnswerBody = {
  session_id: string;
  ask_id?: string;
  answer: "allow_once" | "refuse" | ChosenAnswer[];
};

/** GET /api/plan_usage: Fleet's `PlanUsage` as it is. */
export type { PlanUsage as PlanUsageBody } from "@armada/protocol";

/** GET /api/models: Fleet's `ModelChoices` as it is. */
export type { ModelChoices as ModelsBody } from "@armada/protocol";

/**
 * The Job acts, each forwarded to Fleet unchanged and answered with the `SessionRecord`:
 * `/api/sessions/begin` (`BeginSession`), `/api/sessions/stop`, `/pause`, `/resume` and `/close` (`{ session_id }`),
 * `/api/sessions/tune` (`TuneSession`), `/api/sessions/waiting/dismiss` (`DismissWaiting`),
 * `/api/sessions/sketch/answer` (`AnswerSketch`), `/api/sessions/document/answer` (`AnswerDocument`).
 * `/api/sessions/run/start` (`StartSessionRun`) answers 202 with `SessionRunUnderway`.
 */
export type JobOnly = {
  session_id: string;
};

/** POST /api/sessions/fork `{ session_id }`. 201 with the new Job. */
export type ForkedBody = {
  session: PhoneSession;
};

/** GET /api/repositories: the labels `DispatchBody.repository` may be. */
export type RepositoriesBody = string[];

/** POST /api/sessions. 201 with the Session the text started; 400 for an unknown repository. */
export type DispatchBody = {
  text: string;
  repository: string;
};
export type DispatchedBody = {
  session: PhoneSession;
};

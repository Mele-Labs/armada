// What the Phone Gateway sends the phone (crates/pocket/src/phone_sessions.rs, live.rs).
// Hand-mirrored: the Gateway trims Fleet's Session to an allowlist, so this is not
// `@armada/protocol`'s SessionRecord and nothing here is generated.

/** One `data:` line of GET /api/live. `resync` means read the lists again. */
export type LiveChange = {
  change: "session" | "resync";
  session_id?: string;
};

export type PhoneChoice = { label: string; description?: string };

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
};

/** GET /api/sessions: the live Sessions only. The phone takes Needs you from the ones waiting. */
export type SessionsBody = { sessions: PhoneSession[] };

/** One question's choice: the option labels, or the person's own words. */
export type ChosenAnswer = { question: string; chosen: string[] };

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

/** GET /api/repositories: the labels `DispatchBody.repository` may be. */
export type RepositoriesBody = string[];

/** POST /api/sessions. 201 with the Session the text started; 400 for an unknown repository. */
export type DispatchBody = { text: string; repository: string };
export type DispatchedBody = { session: PhoneSession };

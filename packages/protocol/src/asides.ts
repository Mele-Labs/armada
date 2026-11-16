// An aside: a side question put to a Job without interrupting it, answered by one read-only turn on a thrown-away copy of
// its conversation. `crates/ipc/src/asides.rs`. Since 15 Nov 2026.
//
// The header rules in `protocol.ts` hold here. An aside is kept beside the Job's record and never in its thread, so the
// Job's agent does not see it until the person tells it (`POST /sessions/aside/tell`).

/** How far an aside has got. `failed` carries Fleet's sentence in `Aside.failed`. */
export type AsideState = "asking" | "answered" | "failed";

/** One aside, whole. `session.aside` carries it, so a client replaces the one it holds by `aside_id`. */
export type Aside = {
  aside_id: string;
  session_id: string;
  question: string;
  /** The text the person selected in the thread when he asked. */
  quote?: string;
  /** What the copy has said so far, whole once `state` is `answered`. Empty until it says something. */
  answer: string;
  state: AsideState;
  /** Why a failed aside failed, in Fleet's words. */
  failed?: string;
  asked_at: string;
  answered_at?: string;
  /** When the person told it to the Job. Absent: not told. */
  told_at?: string;
  /**
   * What the copy has looked at so far, in order, one `<Tool> <target>` each: `Read <file>`, `Grep <pattern>` (and
   * ` in <path>` where it gave one) or `Glob <pattern>`. Paths inside the Job's directory are relative to it. Always
   * sent; empty until the copy reads something.
   */
  reads: string[];
};

/** `POST /sessions/aside`. Answered 202 with the new `Aside`, `asking`. */
export type AskAside = {
  session_id: string;
  question: string;
  quote?: string;
};

/** `POST /sessions/aside/tell`. Sends an answered aside to the Job as the person's message, once. Answered with the `Aside`, `told_at` set. */
export type TellAside = {
  session_id: string;
  aside_id: string;
};

/** What `GET /sessions/asides?session_id=` answers: the Job's asides, oldest first. */
export type AsideList = {
  session_id: string;
  asides: Aside[];
};

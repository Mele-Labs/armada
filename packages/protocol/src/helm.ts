// One repository's Helm, as TypeScript sees it. `crates/ipc/src/helm.rs`.
//
// **Helm is a hosted Session** (`docs/concepts/helm.md`, the decision record of 13 Oct 2026).
// `AskHelm`, `HelmSession`, `HelmContext` and `HelmScreen` are the wire the dock speaks. The
// socket Helm used to have (`GET /helm/observe`, `POST /helm/tune`, `GET /helm/file`,
// `helm.changed_checkout`) is gone from Fleet and from this file. `HelmAsked`, `HelmFresh`,
// `HelmUnanswered` and `Freshness` stay for the Commander, whose thread is still Helm's old host.
//
// Hand-written like `turn.ts`: the codegen that would emit both sides does not exist yet.

import type { SentFile, SessionTag, SessionThread, SessionUpload } from "./hosted-sessions";
import type { SessionRecord } from "./sessions";

/**
 * `POST /helm/ask` — what a person says to Helm. **The text may be blank when `attachments` or `mentions`
 * carries something**; a message with none of the three is not sent.
 */
export type AskHelm = {
  text: string;
  /** Where the person is in Bridge, sent with every ask. `#1075`. */
  context?: HelmContext;
  /** Files and pictures the message carried. Left out when there are none. */
  attachments?: SessionUpload[];
  /** What the message names with `@` and `#`. Left out when there are none. */
  mentions?: SessionTag[];
  /**
   * The model and effort for the Helm Session, where this message is the one that starts it. Absent
   * is the Session's default. Ignored once a Session is live, which `tune_session` changes. 422
   * `fleet.unacceptable_effort`.
   */
  model?: string;
  effort?: string;
};

/**
 * `GET /helm/session?manifest_id=` and what `POST /helm/start_fresh` answers: the repository's live Helm
 * Session and its thread, or none. **`session` is absent before the first message and after a person
 * starts fresh.** The dock reads the thread here, then follows `session.row` and `session.changed` on
 * `/events` by the session's id, as a Session's page does. `POST /helm/ask` and `POST
 * /helm/thought_sent` answer a `SessionRecord`, with `helm: true`.
 */
export type HelmSession = {
  manifest_id: string;
  session?: SessionThread;
};

/** What `POST /helm/ask` answers: the Helm Session's record. Its reply arrives as `session.row`. */
export type HelmAnswered = SessionRecord;

/**
 * Where the person is in Bridge, as one ask carries it — a snapshot of the
 * moment it was sent, never a subscription. `crates/ipc/src/helm.rs`.
 */
export type HelmContext = {
  screen: HelmScreen;
  /** The repository the rail has picked. Absent for All repositories. */
  picked?: string;
  /** The Studio open on the Studios surface. Absent on its list, where `screen` is still `studio`. Since 14.7, #1287. */
  studio?: string;
  /** The node selected on that Studio's whiteboard. Only with `studio`. Since 14.7. */
  node?: string;
};

/** Which screen is showing. `App.tsx` is the one place that decides between them. */
export type HelmScreen =
  | "overview"
  | "manifest"
  | "cleanup"
  | "studio"
  /** Kit: the MCP servers a drone is handed. Since 15.1, #1275. */
  | "kit"
  /**
   * This machine's own settings. **Added with `kit` and not before it**:
   * #1287 added `studio` and left this one out, so a person on Settings was
   * reported to Helm as being on the Board.
   */
  | "settings"
  /** The Workflow creator. Since 23.34. */
  | "workflows";

/** What a person said, as Fleet took it, and what it carried. */
export type HelmAsked = {
  ts: string;
  text: string;
  /** Files and pictures it carried, kept by Fleet. */
  files?: SentFile[];
  /** What it named with `@` and `#`. */
  tags?: SessionTag[];
};

/** The stored session could not be resumed, so this reply starts a new one. */
export type HelmFresh = {
  ts: string;
  because: Freshness;
};

/** Why a conversation started over without a person asking it to. Left as a plain union, `Voice`'s reason: no registry declares the set. */
export type Freshness = "session_not_found";

/** No reply came, and why. */
export type HelmUnanswered = {
  ts: string;
  why: string;
};

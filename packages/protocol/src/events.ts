// The event vocabulary, as TypeScript sees it. `crates/ipc/src/event.rs`.
//
// **Split out of `protocol.ts`, not written apart from it.** That file reached
// the 900 lines the gate refuses, and this is the one seam inside it that is
// already a seam on the Rust side: a message pushed over the socket, rather
// than a shape answered to a request. `protocol.ts` re-exports every name here,
// so nothing that imported one had to change.
//
// The header rules there hold here: these are hand-written, they drift the day
// a field moves, and every closed set is left as `string`.

import type { ManifestReading } from "./reading";
import type { MergeLines } from "./merge-lines";
import type { ReviewRun } from "./review-runs";
import type { ReviewWalk } from "./review-walks";
import type { ModList } from "./mods";
import type { SettingsList } from "./settings";
import type { CheckoutRunRecord } from "./rehearsal";
import type { AppRun } from "./app-runs";
import type { ServerState } from "./servers";
import type { SessionRowChanged } from "./hosted-sessions";
import type { ModelDownshiftState } from "./model-downshift";
import type { Projects } from "./projects";
import type { AgentSignIn } from "./agent-sign-in";
import type { PlanUsage } from "./plan-usage";
import type { SleepState } from "./sleep";
import type { TimelineAppended } from "./timeline";
import type { ActFailed } from "./act-failed";
import type { Autopilot } from "./autopilot";
import type { WatchesChanged } from "./watches";
import type { BugReportsChanged } from "./bug-reports";
import type { BrowserRequest, JobTerminal, SessionRecord } from "./sessions";
import type { RepositoryList } from "./setup";
import type { Studio, StudioDeleted, StudioHelmActed } from "./studio";
import type { HelmAskingToRun, HelmCallAnswered } from "./helm-calls";
import type { ProtocolId } from "./connection";

/** One message from Fleet to a connected client. `crates/ipc/src/event.rs`. */
export type StreamMessage =
  | ({ message: "resync" } & Resync)
  | ({ message: "event" } & Delivered)
  | ({ message: "missed" } & Missed);

export type Resync = {
  protocol_id: ProtocolId;
  cursor: number;
};

export type Delivered = {
  cursor: number;
  event: Event;
};

/** The bound was reached and the oldest were dropped. Always followed by a resync. */
export type Missed = { dropped: number };

export type Event =
  | ({ kind: "manifest.reread" } & ManifestReading)
  /** The repositories Fleet serves changed; the list now, whole, as `list_repositories` answers. */
  | ({ kind: "repositories.changed" } & RepositoryList)
  /** A merge line moved on disk; every line now, whole, as `get_merge_lines` answers. Since 22.1. */
  | ({ kind: "merge_lines.changed" } & MergeLines)
  /** A review run moved: queued, running, done or failed, whole. Not replayed; `get_review_runs` reads them again. */
  | { kind: "review_run.changed"; run: ReviewRun }
  /** A held walk moved: checking out, starting, ready, failed or closed, whole. Not replayed. */
  | { kind: "review_walk.changed"; walk: ReviewWalk }
  /** The mods on this machine changed; the list now, whole, as `list_mods` answers. */
  | ({ kind: "mods.changed" } & ModList)
  /** settings.json changed, by a save or by hand; every setting now, whole, as `get_settings` answers. */
  | ({ kind: "settings.changed" } & SettingsList)
  /** A run in the main checkout ended. Since 11.9. */
  | ({ kind: "checkout_run.finished" } & CheckoutRunRecord)
  | ({ kind: "server.starting" } & ServerState)
  | ({ kind: "server.serving" } & ServerState)
  | ({ kind: "server.exited" } & ServerState)
  /** A run after any of its servers' three facts, whole; a client replaces the run it holds for that checkout. Output stays `observe_server`'s. */
  | ({ kind: "app_run.changed" } & AppRun)
  /** A Studio after a write to it, whole. Since 14.6. */
  | ({ kind: "studio.changed" } & Studio)
  /** A Studio a person deleted. Since 14.6. */
  | ({ kind: "studio.deleted" } & StudioDeleted)
  /** Helm took one act on a Studio, beside its `studio.changed`. Since 14.7. */
  | ({ kind: "studio.helm_acted" } & StudioHelmActed)
  | ({ kind: "helm.asking_to_run" } & HelmAskingToRun)
  | ({ kind: "helm.call_answered" } & HelmCallAnswered)
  /** A session after any fact about it, whole. Since 23.43. */
  | ({ kind: "session.changed" } & SessionRecord)
  /** One row of a hosted session's thread, appended or replaced by its id. Since 23.49. */
  | ({ kind: "session.row" } & SessionRowChanged)
  /** A Job's shell opened, by the person's `open_terminal` or its agent's first `type_in_terminal`. */
  | ({ kind: "terminal.opened" } & JobTerminal)
  /** A Job's shell ended: it exited, was stopped, or its Job ended. */
  | ({ kind: "terminal.exited" } & JobTerminal)
  /** An agent's act on its Job browser, held by Fleet until Bridge answers it with `answer_browser_request`. Not replayed. */
  | ({ kind: "browser.request" } & BrowserRequest)
  /** Sleep mode after any change to it, whole. */
  | ({ kind: "sleep.changed" } & SleepState)
  /** A repository's Autopilot after any change to it, whole. #2092. */
  | ({ kind: "autopilot.changed" } & Autopilot)
  /** A Job's watches after any change to one, whole, as `list_watches` answers. */
  | ({ kind: "watches.changed" } & WatchesChanged)
  /** Every bug report agents made of Armada, whole, after one was made, kept or dropped. */
  | ({ kind: "bug_reports.changed" } & BugReportsChanged)
  /** The plan's windows after a session read them moving, whole. */
  | ({ kind: "plan_usage.changed" } & PlanUsage)
  /** The default model's step down after any change to it, whole. Absent `downshift` is none. */
  | ({ kind: "model_downshift.changed" } & ModelDownshiftState)
  /** A repository's projects after any write to one, whole, as `list_projects` answers them. */
  | ({ kind: "projects.changed" } & Projects)
  /** The agents' account signed out or back in, or Fleet's sign-in of it started or ended, whole. */
  | ({ kind: "agent_sign_in.changed" } & AgentSignIn)
  /** Fleet kept one entry for the timeline; a client holding a window appends it. Since 23.82. */
  | ({ kind: "timeline.appended" } & TimelineAppended)
  /** A background act failed after its request was answered: the act, what it was for, and the refusal. Not replayed. */
  | ({ kind: "act.failed" } & ActFailed);

// `manifest.reread` carries `ManifestReading` itself rather than a payload
// wrapping it: the answer to `get_manifest_reading` and this event's body are
// the same fact. It names no Drone and no step — a Manifest is Fleet's own — and
// carries no `actor` and no `at` on the envelope for that reason: `at` would be
// a second copy of the reading's own, and nobody pressed anything in Armada at all.

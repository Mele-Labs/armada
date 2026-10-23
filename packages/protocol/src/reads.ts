// The shape of a read, and of what comes back from asking for one.
//
// **A read is not a channel.** These say what Bridge holds — read, reading,
// failed, and what arrived — which is the same shape whatever carried it. The
// channel names and the preload surface that carried it are Electron's and stay
// in `apps/desktop`.

import type { ModPromoted, TrackerTry } from "./mods";
import type {
  CheckOutput,
  ManifestSummary,
  ModelChoices,
  WorkflowSummary,
  LeftOutWorkflow,
  RepositorySummary,
  SlotPoolChanged,
  WireError,
  WorktreesHeld,
  PullRequestState,
} from "./index";
/**
 * `GET /worktrees`, where a surface asked for it.
 *
 * What is being decided here is which of a set to give back, which is a
 * question about the set. The four states are those every read has: "nobody
 * asked" and "the read failed" are different things to draw.
 */
export type HeldWorktrees =
  | { state: "none" }
  | { state: "reading" }
  | { state: "read"; held: WorktreesHeld }
  | { state: "failed"; outcome: Outcome };

/**
 * What one Check's output came back as.
 *
 * **Answered to the caller rather than published as state.** A recorded output
 * never moves, it is fetched by the person who pressed one row, and holding it
 * in `BridgeState` would make one reader's gesture part of what every surface
 * re-renders on — and keep a test runner's whole log alive.
 */
export type CheckOutputRead =
  | { ok: true; output: CheckOutput }
  | { ok: false; outcome: Outcome };

/**
 * What one frame came back as: the bytes, and what they are.
 *
 * **Answered to the caller rather than published as state**, for
 * `CheckOutputRead`'s reasons — a kept frame never moves, and holding one in
 * `BridgeState` would keep hundreds of kilobytes alive.
 *
 * **Bytes and not a string.** Every other read on this seam is JSON; this one
 * is a file, and it crosses to the renderer as the array Electron's structured
 * clone already carries. A base64 in between would inflate it by a third and
 * would have to be undone on the other side to draw it.
 *
 * `type` is the media type Fleet answered with, read off the file's own name.
 * It travels with the bytes because the renderer needs it to make a `Blob`, and
 * nothing on that side should be parsing an extension to guess.
 */
export type FrameRead =
  | { ok: true; bytes: Uint8Array; type: string }
  | { ok: false; outcome: Outcome };

/** The values a proposal may name. Empty until the connection answers. */
export type Holdings = {
  workflows: WorkflowSummary[];
  manifests: ManifestSummary[];
  /** `null` until read: an empty roster and an unread one are not the same. */
  models: ModelChoices | null;
  /** The Kit and carried workflows Fleet runs without, each with why. #425. */
  leftOut?: LeftOutWorkflow[];
  /** Every repository Fleet serves, set up or not — the rail's picker. Absent from a Fleet older than the route. */
  repositories?: RepositorySummary[];
};

/** What a command answered. A refusal names itself; it never renders as silence. */
export type Outcome =
  /**
   * `slotChanged` rides on the same terms as every receipt here: it changes no
   * row the board reads, so what came back is shown once by the surface that
   * asked rather than published as app state.
   */
  | {
      ok: true;
      /** What `change_slot_pool` answered: for a `release`, the branch and the files committed to it. */
      slotChanged?: SlotPoolChanged;
      /** The pull request as the forge showed it after a press, or when it was read. */
      pullRequest?: PullRequestState;
      /** What `promote_mod` answered: the branch the mod is on. Shown once, by the surface that pressed. */
      modPromoted?: ModPromoted;
      /** What `try_tracker_mod` answered, passed or not. Drawn on the row that pressed Try. */
      trackerTried?: TrackerTry;
    }
  | { ok: false; why: "not_connected" }
  | { ok: false; why: "not_understood" }
  | { ok: false; why: "empty_brief" }
  | { ok: false; why: "no_manifest" }
  /** A Manifest-scoped act on the picked repository, which has no `armada.yml` yet. Refused before it is sent. */
  | { ok: false; why: "not_set_up" }
  | { ok: false; why: "already_answering_lesson" }
  | { ok: false; why: "already_setting" }
  /**
   * A capture press in the capture window with nothing held — #1294. Bridge's
   * own, and Fleet never sees it: a Note with no capture is not the act.
   */
  | { ok: false; why: "nothing_held" }
  /**
   * A capture in the capture window after the Run stopped serving — #1294.
   * **Refused before it is sent**: a loopback port is not an identity, so a
   * Note taken against whatever replaced the Run would be a record of
   * something other than what it names. `docs/practices/capture-window.md`.
   */
  | { ok: false; why: "run_ended" }
  /**
   * Approve on a capture window that no Session opened. Bridge's own, and Fleet never sees
   * it: a Studio's bar offers no Approve, since there is nobody to tell.
   */
  | { ok: false; why: "no_owner" }
  | { ok: false; why: "refused"; error: WireError }
  | { ok: false; why: "transport"; detail: string; fault: TransportFault };

/**
 * What went wrong between Bridge and Fleet, for the one refusal that carries no
 * `WireError`.
 *
 * **Three answers and not one string.** `detail` beside this is the machine's
 * own words and says nothing a person can act on — "The operation was aborted
 * due to timeout" is the same sentence whether Bridge waited five seconds on a
 * read or two and a half minutes on a model call, and it never says which route
 * it was about. What separates the three is what to do next, which is the
 * distinction a generic transport message throws away:
 *
 * - `timed_out` — the request went out and no answer came back inside the wait.
 *   **It may still have been carried out**, so the board is what says whether
 *   to send it again.
 * - `unreachable` — the fetch itself failed. Whether Fleet read it is unknown.
 * - `unanswerable` — Fleet answered a status, and the body was not a refusal
 *   Bridge could read. Fleet is running and the two disagree about the route.
 *
 * The method and the route travel on every one, because a failure that does not
 * name what was asked is one somebody has to answer with a question.
 */
export type TransportFault = {
  /** What Bridge sent, so the failure names the act rather than the app. */
  method: string;
  /** The route asked, with ids already encoded into it. */
  path: string;
  /**
   * What the request carried, field by field — **only on a route
   * `PENDING_ROUTES` names**, so the debug info a person copies says what the
   * act would have done (`sentOf`).
   */
  sent?: Record<string, string>;
} & (
  | {
      why: "timed_out";
      /** How long Bridge waited, in milliseconds. The dial, not a guess. */
      waitedMs: number;
    }
  | { why: "unreachable" }
  | { why: "unanswerable"; status: number }
);

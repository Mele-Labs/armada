// What the command palette can reach in this app, and what it cannot reach yet.
//
// **Its own file so `App.tsx` does not grow another hundred lines**, and
// because this is one subject: the palette is a superset of the UI, so every
// act in the registry has to be answered here — either by doing it, or by
// saying in a few words why this build cannot.
//
// **The places a person can go are not here.** The rail and the palette are two
// controls over one roster, and it was in this file while the palette was the
// only one of them that drew it. It is `packages/shell/src/surfaces.ts` now,
// beside both.
//
// # A row that cannot act says why, except an act with nothing to act on
//
// The contract's rule about a registered binding nothing answers is that the
// palette may draw it disabled or leave it out, **on a fact rather than on a
// list of exceptions kept in the app**. The registry's own `unbuilt` column is
// that fact for `p` and `X`, and those rows stay drawn. This file is the second
// kind: an act Bridge has, which cannot reach anything at this moment or from
// this surface.
//
// **The acts on one Job are left out** (`absentIn`). Bridge has no page that reads a Job, so
// none of them has anything to act on, and a dimmed row saying so is the first thing the palette
// would show. Copy debug info with no failure on screen is left out too (12 Oct 2026), so
// "debug" ↵ lands on Debug Bridge. Every other dormant row is still drawn and still says why.

import type { StudioNodeByHandKind } from "@armada/components";
import type { PaletteChoice } from "@armada/shell";

/** What the palette says when the act is on a Studio's rail and no Studio is open. */
const NO_STUDIO = "open a Studio first";

/** What the palette says when the act reads from or sends to Fleet and nothing is connected. */
export const FLEET_DOWN = "Fleet is not connected";

/** What the palette says of Debug Bridge where the annotation layer is not loaded, as in a packaged app. */
const NOT_DEV = "needs Bridge under pnpm dev";

/** The acts the palette never offers: they act on a focused row, which it has none of. */
const ON_A_ROW = ["open", "close"] as const;

/**
 * The acts the palette leaves out rather than dims: an act on a focused row; and Copy debug info with no
 * failure on screen, which the owner had out of the way on 12 Oct 2026 so that "debug" ↵ is Debug Bridge.
 */
export function absentIn(where: { failing?: boolean }): readonly string[] {
  return where.failing === false ? [...ON_A_ROW, "copy_debug_info"] : ON_A_ROW;
}

/**
 * Why each act cannot be chosen, by action id. An id absent from the answer is
 * an act this build can do.
 *
 * `unbuilt` rows are not here: the registry already carries their reason and
 * the shell reads it, so repeating them would be the second list this file
 * exists to avoid.
 */
export function dormantIn(where: {
  /** Whether a failure is on screen for Copy debug info to copy. */
  failing: boolean;
  /** Whether Fleet is connected, which is what Refresh reads from. */
  live: boolean;
  /** Whether this Bridge runs the annotation layer, which is what Debug Bridge shoots and sends through. Absent is no. */
  debuggable?: boolean;
  /**
   * Why the open Studio's Run is off, `undefined` where it is on, `null` with
   * no Studio's rail drawn — `@armada/screens`' `useStudioRunOff`.
   */
  studioRun?: string | null;
  /** The same for the rail's four Add kinds — `useStudioAddOff`. */
  studioAdd?: string | null;
}): Readonly<Record<string, string | undefined>> {
  return {
    // Copy debug info takes a `Failure`, and a healthy Job has none — the
    // finding `keys.ts` recorded when it claimed `c` and reached nothing. The
    // palette can do better than the row could: where a failure *is* on
    // screen, this is the act that copies it.
    copy_debug_info: where.failing ? undefined : "no failure on screen to copy",

    // The rail's Note, Link, Sketch and Zone, which these rows arm as their
    // keys do, and Run, which its row opens as `R` does — 2 Oct 2026. Each is
    // off for the rail's own reason, so the two say one thing.
    add_note: where.studioAdd === null ? NO_STUDIO : where.studioAdd,
    add_link: where.studioAdd === null ? NO_STUDIO : where.studioAdd,
    add_sketch: where.studioAdd === null ? NO_STUDIO : where.studioAdd,
    add_zone: where.studioAdd === null ? NO_STUDIO : where.studioAdd,
    start_studio_run: where.studioRun === null ? NO_STUDIO : where.studioRun,

    // Global acts with no surface behind them.
    //
    // Helm's dock is built (#944) and `⌘J` opens it, but that state is
    // `Shell.tsx`'s own `useDock` and `App` holds no handle to it — a wiring
    // gap, same shape as the four signposts above.
    helm: "⌘J opens it; not reachable from the palette yet",
    dispatch_from_field: "n does this; ⌘N is for a field",
    // Not a wiring gap: ⌘Enter sends whatever is typed in the field that has
    // focus, and opening the palette took that focus away. The row is drawn so
    // the binding is discovered, and it says what it needs.
    send_message: "type in a message box first",
    // The same shape: ⇧⌘Enter saves what the Session dispatcher holds, and
    // only that sheet has a draft to save.
    queue_session: "in a Job's dispatcher",
    // The key opens the Dispatch panel on a Session with an `@` typed; the palette has no request to type it into.
    dispatch_after: "⌥A does this",
    // The key puts the caret in the open Job's message box, which the palette's own field has just taken it from.
    message_job: "⌥M does this",
    // The Board's own field answered this and went with the Board. This
    // palette is the search now, so a row reopening it would be circular.
    search: "this is the search",
    // `history.ts` answers ⌘[ and ⌘] from the window, and the palette has no handle to it.
    history: "⌘[ and ⌘] go back and forward",
    refresh: where.live ? undefined : FLEET_DOWN,
    debug_bridge: where.debuggable !== true ? NOT_DEV : where.live ? undefined : FLEET_DOWN,
  };
}

/** What the app has to be able to do for a chosen row to mean anything. */
export type PaletteHands = {
  openSession: (sessionId: string) => void;
  compose: () => void;
  /**
   * Go to a place in Navigation, by the id the surfaces list gave it.
   *
   * **It takes the id, and it did not have to before.** With one destination
   * in that section every row meant the same thing, so the argument would have
   * been ignored; with two, a row that ignored it would go to the wrong screen
   * — silently, and only for whoever chose the second one.
   */
  surface: (surfaceId: string) => void;
  /** Requests, which the roster lists as a place and which opens the side panel. Absent, its row goes to `surface` like any other. */
  requests?: () => void;
  /**
   * Go to the Manifest surface with one Check or Command picked.
   *
   * **It selects; it does not run.** Journey 9 has the palette opening the
   * surface with that entry chosen, and a row that started a destructive
   * Command straight off a list of forty would be an act nobody confirmed —
   * which is the one confirmation that surface exists to carry.
   */
  run: (entryId: string) => void;
  copyDebugInfo: () => void;
  /**
   * Go to the settings row's own screen, by the id `settings` was given.
   *
   * **One row per category of Settings.** `fleet_settings` and each
   * `settings:<category>` go to the Settings screen at that category; the
   * switch lives with the app that owns navigation rather than here, the way
   * `surface` and `run` already read their id against the screen that
   * answers for it.
   */
  openSetting: (id: string) => void;
  /** Shoot the window and start a Session debugging it — `⇧⌘D`'s act, carried out by the annotation layer. */
  debugBridge: () => void;
  /** Re-read everything Fleet holds — `⇧⌘R`'s act. */
  refresh: () => void;
  /** Pick a repository, as the title row's picker does; `null` is All repositories. */
  pickRepository: (root: string | null) => void;
  /** Open the open Studio's Run menu on its rail, as a press on Run does. */
  studioRun: () => void;
  /** Arm a kind on the open Studio's rail, as its key does. */
  studioAdd: (kind: StudioNodeByHandKind) => void;
  /** Open the box that starts a Mod Job from his words. */
  createMod: () => void;
};

/** The palette's rows about the window as a whole, which no registry row answers. */
export const CREATE_MOD = "create_mod";

/** The palette's Add rows, by the kind each arms on the rail. */
const ADD_ROW: Readonly<Record<string, StudioNodeByHandKind>> = {
  add_note: "note",
  add_link: "link",
  add_sketch: "sketch",
  add_zone: "zone",
};

/**
 * Carry out a chosen row.
 *
 * **Nothing here decides whether the palette closes.** Choosing a row closes
 * it, which is the component's rule and the right one — every act below either
 * moves the screen or puts the cursor somewhere, and a list still covering
 * what you just asked for is a list you have to dismiss twice.
 */
export function carryOut(choice: PaletteChoice, hands: PaletteHands): void {
  switch (choice.of) {
    case "session":
      hands.openSession(choice.id);
      return;
    case "surface":
      if (choice.id === "requests" && hands.requests !== undefined) hands.requests();
      else hands.surface(choice.id);
      return;
    case "run":
      hands.run(choice.id);
      return;
    case "setting":
      hands.openSetting(choice.id);
      return;
    case "repository":
      hands.pickRepository(choice.root);
      return;
    case "board":
      if (choice.id === CREATE_MOD) hands.createMod();
      return;
    case "act":
      act(choice.id, hands);
  }
}

function act(id: string, hands: PaletteHands): void {
  switch (id) {
    case "new_job":
      hands.compose();
      return;
    case "copy_debug_info":
      hands.copyDebugInfo();
      return;
    case "refresh":
      hands.refresh();
      return;
    case "debug_bridge":
      hands.debugBridge();
      return;
    case "start_studio_run":
      hands.studioRun();
      return;
    case "add_note":
    case "add_link":
    case "add_sketch":
    case "add_zone":
      hands.studioAdd(ADD_ROW[id]!);
      return;
    default:
      // An act the registry carries and this build does not answer. `dormantIn`
      // drew it dimmed and the palette refuses a dormant row before it reaches
      // here, so this arm holds only while the two disagree — which is what an
      // act added to one and not the other looks like.
      return;
  }
}

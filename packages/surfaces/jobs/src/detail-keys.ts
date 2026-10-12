// Job detail's half of the keyboard map, read off `actions.toml`.
//
// The Board's half is `keys.ts` and the two are deliberately separate: they
// answer different keys on different objects, and one handler branching on
// which surface is up is how a key ends up doing two things. What they share is
// the safety rules, and those are imported rather than restated —
// `holdsText` is the whole of "single-key shortcuts are suppressed while a text
// input holds focus", and typing "fig" into a redirect box must not open a
// diff, open a gate.
//
// # The contextual tier, and what detail does with each key
//
// | Key | `actions.toml` | Here |
// |---|---|---|
// | `f` | `open_diff`, scope `detail` | open the diff sheet |
// | `b` | `report_job`, scope `detail` | open the dialog that says this job failed in error |
// | `r` | `run`, scope `detail` | open the run sheet, nothing selected — Journey 9 |
// | `n` | `new_job`, scope `anywhere` | open the composer — the Board's own key, answered the same way here |
// | `Esc` | `back`, scope `detail` | the list, and `App.tsx` owns it |
//
// # No cursor, and no keys that move one
//
// `j` `k` `↓` `↑` moved between a log's rows and `h` `l` `←` `→` opened the
// focused row's payload. Both went when the activity log did (#1761): nothing
// on the Overview draws a log now, so they reached nothing, and the arrows are
// the browser's again. `j`/`k` are still `move_focus` on the Board's list, in
// `keys.ts`.

import { useEffect, useRef } from "react";

import { isPressed } from "@armada/components";

import { holdsText } from "@armada/screens/src/keys";

/* `FLEET_LOG` was here, naming the region that drew what Fleet did to the Job
   itself. That region is gone: it and the holdings card both answered *what is
   happening on this machine right now*, so the tail moved into
   `JobHoldsSummary` below the run and the standalone region went with it. The
   constant outlived the thing it named by one change, which is how a keyboard
   map ends up addressing regions nobody draws. */

/** What a press on job detail means. `null` is a key this surface does not carry. */
export type DetailPress =
  /** `f` — open the Job's patch, on the layer that can hold it. */
  | { act: "diff" }
  /** `o` — open a Check's output, on the layer that can hold it. */
  | { act: "output" }
  /** `b` — say this job failed in error. */
  | { act: "report" }
  /** `B` — give this job a higher cost ceiling. */
  | { act: "raise" }
  /** `T` — let this job take more turns. */
  | { act: "raiseTurns" }
  /** `r` — open the run sheet, nothing selected. */
  | { act: "run" }
  /** `n` — open the composer. Scope `anywhere`, so detail answers it too. */
  | { act: "compose" };

/**
 * What a keypress means on job detail, or `null` for nothing.
 *
 * **Every suppression is here rather than at the call site**, so there is one
 * place the safety rules can be checked against the contract instead of five
 * places they can each be forgotten in. `Escape` is not among them: it belongs
 * to the surface that opened the Job, and two handlers answering one key is the
 * defect `answersEnter` exists to prevent one key over.
 */
export function detailPressOf(event: KeyboardEvent): DetailPress | null {
  if (holdsText(event.target)) return null;
  if (event.repeat) return null;

  // Each act's keys are `keymap`'s, which compares modifiers exactly — so a bare
  // key never fires under a ⌘ chord, and one a person rebound answers here.
  for (const [id, press] of DETAIL_ACTS) {
    if (isPressed(id, event)) return press;
  }
  return null;
}

/** The registry's act for each press, in the order a shared key would be decided. */
const DETAIL_ACTS: readonly (readonly [string, DetailPress])[] = [
  ["open_diff", { act: "diff" }],
  ["open_output", { act: "output" }],
  ["report_job", { act: "report" }],
  ["raise_cost_cap", { act: "raise" }],
  ["raise_turn_cap", { act: "raiseTurns" }],
  ["run", { act: "run" }],
  ["new_job", { act: "compose" }],
];

/**
 * What the keyboard can name on the screen, as the surface built it.
 *
 * **Data, not selectors.** Every act below reaches its target through one of
 * these lists, so a chapter that cannot open or a phase that draws no card is
 * something the compiler knows about rather than something a query happens to
 * miss.
 */
export type DetailShape = {
  /**
   * Open the trailing sheet `f` names — the Job's patch.
   *
   * **The one act here that opens a layer rather than a chapter.** The patch
   * stopped being something the panel draws, so the key that opened a chapter
   * in place opens the sheet instead, and the screen passes in how rather than
   * this file learning what a sheet is.
   *
   * Absent leaves the press unswallowed, as `onReport` does.
   */
  onOpenSheet?: () => void;
  /**
   * Open a Check's output on the trailing layer — `o`.
   *
   * **Required, not optional.** A press that finds no handler answers nothing,
   * which looks exactly like a key that is not bound — and that shipped once,
   * for the log's `L`, wired everywhere except at the one call site. Required
   * makes that a compile error, and a screen with nothing to open passes a
   * function that does nothing.
   */
  onOpenOutput: () => void;
  /**
   * Open the run sheet, nothing selected — `r`, Journey 9. `onOpenOutput`'s
   * shape and its reason: a sheet this file does not build, on a layer the
   * screen owns.
   */
  onOpenRun: () => void;
  /**
   * Open the report dialog. **The one entry here that moves nothing on the
   * screen** — every other act opens something this file already holds, and
   * this one raises a dialog the screen owns, so the screen passes in what to
   * call rather than this file learning what a report is.
   *
   * Absent where the state does not offer it, and the press is then left
   * unswallowed rather than answered with nothing.
   */
  onReport?: () => void;
  /**
   * Open the raise dialog — `B`. **The second entry here that moves nothing on
   * the screen**, and it takes `onReport`'s shape for that reason: the screen
   * owns the dialog and passes in what to call.
   *
   * Absent where the job is not being held for money, and the press is then
   * left unswallowed rather than answered with nothing.
   */
  onRaiseCap?: () => void;
  /**
   * Open the turn-cap dialog — `T`. `onRaiseCap`'s shape and its rule, on the
   * other ceiling.
   *
   * Absent where the job is not being held for turns, and the press is then
   * left unswallowed rather than answered with nothing. **Never present beside
   * `onRaiseCap`**: `budget_hold` names one ceiling, so a job answers one of
   * these two keys and not both.
   */
  onRaiseTurnCap?: () => void;
  /**
   * Open the composer — `n`. **Required, unlike `onReport` and the two
   * ceilings above**: those answer only where the render offers the act, and
   * `new_job`'s scope is `anywhere`, so this is always offered rather than
   * conditional on the Job's state.
   */
  onCompose: () => void;
};

/**
 * Bind the detail's contextual tier.
 *
 * The listener is bound once, for the life of the open Job, and reads the
 * screen through a ref: the run and the story are rebuilt on every tick of the
 * clock, and a listener re-bound sixty times a minute is a listener that is
 * sometimes not bound at all.
 */
export function useDetailKeys(shape: DetailShape): void {
  const held = useRef(shape);
  useEffect(() => {
    held.current = shape;
  });

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      const press = detailPressOf(event);
      if (press === null) return;
      if (act(press, held.current)) event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);
}

/**
 * Carry out a press.
 *
 * **`false` where nothing was there to act on**, so the caller knows not to
 * swallow the key — `f` on a Job with no diff should leave the browser's own
 * behaviour alone rather than silently eating the press.
 */
function act(press: DetailPress, shape: DetailShape): boolean {
  switch (press.act) {
    case "diff":
      return diff(shape);
    case "output":
      return sheet(shape.onOpenOutput);
    case "report":
      return report(shape);
    case "raise":
      return raise(shape);
    case "raiseTurns":
      return raiseTurns(shape);
    case "run":
      return sheet(shape.onOpenRun);
    case "compose":
      shape.onCompose();
      return true;
  }
}

/**
 * Open one of the trailing sheets, where the screen gave a way to.
 *
 * **One spelling for two keys.** `o` and `r` differ only in which reading they
 * open, and two near-identical functions is two places for the swallow rule to
 * drift apart.
 */
function sheet(open: (() => void) | undefined): boolean {
  if (open === undefined) return false;
  open();
  return true;
}

/**
 * `b`, from `actions.toml` — `report_job`, scope `detail`, which is this
 * surface and no other. It confirms, and the dialog it raises is the
 * confirmation: nothing is filed by the press.
 */
function report(shape: DetailShape): boolean {
  if (shape.onReport === undefined) return false;
  shape.onReport();
  return true;
}

/**
 * `B`, from `actions.toml` — `raise_cost_cap`, scope `detail`. It confirms, and
 * the dialog it raises is the confirmation: nothing is sent by the press.
 *
 * **Absent on every job that is not held for money**, which is `report`'s rule
 * for a different reason: that act is offered on one render, and this one is
 * offered where the screen is drawing the refusal it answers.
 */
function raise(shape: DetailShape): boolean {
  if (shape.onRaiseCap === undefined) return false;
  shape.onRaiseCap();
  return true;
}

/**
 * `T`, from `actions.toml` — `raise_turn_cap`, scope `detail`. It confirms, and
 * the dialog it raises is the confirmation: nothing is sent by the press.
 *
 * **Absent on every job that is not held for turns**, which is `raise`'s rule
 * on the other ceiling. The two are never both present: a job over budget is
 * over one of them, and the key that cannot start it is left unbound.
 */
function raiseTurns(shape: DetailShape): boolean {
  if (shape.onRaiseTurnCap === undefined) return false;
  shape.onRaiseTurnCap();
  return true;
}

/**
 * Open the Job's patch on the layer that can hold it.
 *
 * **It opens a sheet rather than a chapter**, because a patch in a 602px column
 * is a decision taken on a line that wrapped. The screen holds which sheet is
 * open and closing is `Esc`, so this is one direction only — pressing `f` twice
 * does not toggle, where pressing it twice used to close the chapter.
 */
function diff(shape: DetailShape): boolean {
  if (shape.onOpenSheet === undefined) return false;
  shape.onOpenSheet();
  return true;
}

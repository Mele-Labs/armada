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
// | `j` `k` `↓` `↑` | `move_focus`, scope `list and detail` | move between the rows of the log the cursor is in |
// | `h` `l` `←` `→` | `disclose`, scope `detail` | open and close the focused log entry's payload |
// | `f` | `open_diff`, scope `detail` | open the diff sheet |
// | `b` | `report_job`, scope `detail` | open the dialog that says this job failed in error |
// | `r` | `run`, scope `detail` | open the run sheet, nothing selected — Journey 9 |
// | `n` | `new_job`, scope `anywhere` | open the composer — the Board's own key, answered the same way here |
// | `Esc` | `back`, scope `detail` | the list, and `App.tsx` owns it |
//
// # It names what it opens, and holds what it opened
//
// This file used to find its controls by the class names the components ship —
// `.armada-srow__name`, `button.armada-entry`, `.armada-story__chapter`,
// `button.armada-phases__control` — and press them. It worked, and it was a
// component's internals leaking into the app: a class rename broke the keyboard
// with nothing to catch it, because typecheck cannot see a string selector.
// #271.
//
// **The state moved here instead.** This file holds which log row is showing
// its payload, and the screen passes it down. Opening a thing is naming it now,
// and a name is a value the compiler reads.
//
// # Focus is still the cursor, because nothing draws another one
//
// `j`/`k` move between rows and a row is a control; there is no prop for "the
// row the keyboard is on", and a cursor held in state that nothing renders is a
// cursor nobody can see. So the rows the cursor roves are still found in the
// document — but by a name **this app writes itself**: the payload id `Log`
// gives every row. A rename of it is a rename in this package, and it is one
// the compiler follows.
//
// Hover on the strip is not among them and never will be: it reports where the
// pointer is, not what a reader decided.

import { useEffect, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";

import { holdsText } from "./keys";
import { LOG_REGION, rowOfPayload } from "./Log";

/* `FLEET_LOG` was here, naming the region that drew what Fleet did to the Job
   itself. That region is gone: it and the holdings card both answered *what is
   happening on this machine right now*, so the tail moved into
   `JobHoldsSummary` below the run and the standalone region went with it. The
   constant outlived the thing it named by one change, which is how a keyboard
   map ends up addressing regions nobody draws. */

/** What a press on job detail means. `null` is a key this surface does not carry. */
export type DetailPress =
  /** `j` `k` `↓` `↑` — move the cursor within whichever list holds it. */
  | { act: "move"; by: 1 | -1 }
  /** `h` `l` `←` `→` — open or close what the focused row holds. */
  | { act: "disclose"; open: boolean }
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
  // A modifier means a different tier is being addressed — the palette's `⌘K`,
  // the rail's `⌘1`–`⌘5`, and `⌘[` `⌘]` for back and forward.
  if (event.metaKey || event.ctrlKey || event.altKey) return null;
  if (holdsText(event.target)) return null;
  // A held key repeats. Only movement accepts one — a repeat that opened
  // fourteen diffs is a repeat nobody asked for.
  const moves =
    event.key === "j" || event.key === "k" || event.key === "ArrowDown" || event.key === "ArrowUp";
  if (event.repeat && !moves) return null;

  switch (event.key) {
    case "j":
    case "ArrowDown":
      return { act: "move", by: 1 };
    case "k":
    case "ArrowUp":
      return { act: "move", by: -1 };
    case "l":
    case "ArrowRight":
      return { act: "disclose", open: true };
    case "h":
    case "ArrowLeft":
      return { act: "disclose", open: false };
    case "f":
      return { act: "diff" };
    case "o":
      return { act: "output" };
    case "b":
      return { act: "report" };
    // Shifted, because plain `b` is the report above: the lowercase mnemonic is
    // spent, and a mistyped filing key must not open a dialog about money.
    case "B":
      return { act: "raise" };
    // Shifted, on `B`'s terms: plain `t` is unbound today and a lowercase key
    // that opened a dialog about a ceiling would be a mistype away from one.
    case "T":
      return { act: "raiseTurns" };
    // Plain, freed by `actions.toml`'s narrowing of `review` to scope `list` —
    // #624. `Journey 9` and the Board's `r` never contend: one is the list's
    // key on a row, this is detail's on the whole screen.
    case "r":
      return { act: "run" };
    // `new_job`'s scope is `anywhere`, not `detail` — the one contextual key
    // that acts on nothing on screen, so detail answers it exactly as the
    // Board does rather than leaving it for a surface with a cursor on it.
    case "n":
      return { act: "compose" };
    default:
      return null;
  }
}

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

/** The open state this file holds, in the shape the screen takes it in. */
export type DetailKeys = {
  /**
   * What one log takes: its own name, the row of *its* rows that is open, and
   * how to say one was pressed. **Per log**, so two logs over one stream,
   * which hold equal ids, never open a row in both at once.
   */
  inLog: (region: string) => {
    region: string;
    openId: string | null;
    onOpen: (rowId: string | null) => void;
  };
};

/** A row of a log, which is a row and the log it is in. */
type OpenEntry = { region: string; row: string };

/**
 * Bind the detail's contextual tier, and hold what it opened.
 *
 * The listener is bound once, for the life of the open Job, and reads the
 * screen through a ref: the run and the story are rebuilt on every tick of the
 * clock, and a listener re-bound sixty times a minute is a listener that is
 * sometimes not bound at all.
 */
export function useDetailKeys(shape: DetailShape): DetailKeys {
  const [openEntry, setOpenEntry] = useState<OpenEntry | null>(null);

  const held = useRef(shape);
  useEffect(() => {
    held.current = shape;
  });

  const moves = useRef<Moves>({
    entry: setOpenEntry,
  });

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      const press = detailPressOf(event);
      if (press === null) return;
      if (act(press, held.current, moves.current)) event.preventDefault();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  return {
    inLog: (region) => ({
      region,
      openId: openEntry?.region === region ? openEntry.row : null,
      onOpen: (row) => setOpenEntry(row === null ? null : { region, row }),
    }),
  };
}

/** What a press can move. Passed as one so `act` stays a function. */
type Moves = {
  entry: Dispatch<SetStateAction<OpenEntry | null>>;
};

/**
 * Carry out a press.
 *
 * **`false` where nothing was there to act on**, so the caller knows not to
 * swallow the key — `f` on a Job with no diff should leave the browser's own
 * behaviour alone rather than silently eating the press.
 *
 * Every mover takes the updater form. A state updater runs twice under
 * StrictMode, so each one is a pure reading of what was open and nothing else
 * happens inside them.
 */
function act(press: DetailPress, shape: DetailShape, on: Moves): boolean {
  switch (press.act) {
    case "move":
      return move(press.by);
    case "disclose":
      return disclose(press.open, on);
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

/** A row the cursor can be on: what it is called, and the control it is. */
type Row = { id: string; control: HTMLElement };

/** A row of a log, which is one of those and the log it was drawn in. */
type Entry = Row & { region: string };

/**
 * The log's rows, in document order and across every log the story is drawing.
 * A row is named by the payload it points at, which `Log` writes and nothing
 * else reads.
 */
function entryRows(): Entry[] {
  return [...document.querySelectorAll<HTMLElement>(`[${LOG_REGION}] button`)].flatMap(
    (control) => {
      const id = rowOfPayload(control.getAttribute("aria-controls"));
      const region = control.closest(`[${LOG_REGION}]`)?.getAttribute(LOG_REGION);
      return id === null || region == null ? [] : [{ id, control, region }];
    },
  );
}

/** Whether a row is the one the cursor is on. */
function under(row: Row): boolean {
  const focused = document.activeElement;
  return row.control === focused || (focused !== null && row.control.contains(focused));
}

/**
 * Move the cursor between the rows of the log that holds it. Focus outside a
 * log is left alone, and the press with it.
 */
function move(by: 1 | -1): boolean {
  const inLog = document.activeElement?.closest(`[${LOG_REGION}]`) != null;
  if (!inLog) return false;
  const rows = entryRows();
  if (rows.length === 0) return false;
  const at = rows.findIndex(under);
  // Wrapping is deliberate: these are short lists a person roves rather than
  // scrolls, and a cursor that stops dead at the end of six rows is a cursor
  // somebody presses `j` at twice.
  const next = at < 0 ? 0 : (at + by + rows.length) % rows.length;
  rows[next]?.control.focus();
  return true;
}

/**
 * Open or close the payload of the log entry under the cursor.
 *
 * **It is named, never pressed.** A payload already open stays open on
 * `l` and closes on `h`; pressing regardless would make the two keys one
 * toggle, which is what having two of them is for avoiding.
 */
function disclose(open: boolean, on: Moves): boolean {
  if (!(document.activeElement instanceof HTMLElement)) return false;

  const entry = entryRows().find(under);
  if (entry !== undefined) {
    const now = { region: entry.region, row: entry.id };
    on.entry((was) =>
      open ? now : was?.region === now.region && was.row === now.row ? null : was,
    );
    return true;
  }
  return false;
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

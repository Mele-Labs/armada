// What main reads of a walk playing in the page, and the five things it does to it —
// `docs/practices/capture-window.md`, *A walk in the page*.
//
// **`locate.ts`'s terms**: every source travels with the ask and leaves nothing behind. The page's
// answer is a string main parses and bounds here, and the acts are a closed list main chooses
// from, so the bar names one of five and never a script.

import type { CaptureWalk } from "../../shared/capture-window";

/** The most the page may answer with. A walk's state is a name, a caption and six small fields. */
export const WALK_RAW_MAX = 4096;
/** The most steps a walk is believed to have, and the longest a name or a caption is drawn. */
export const WALK_STEPS_MAX = 500;
const NAME_MAX = 80;
const SAID_MAX = 240;

/** The acts main may call on the page's walk: `next`, `back`, `replay` and autoplay on or off. */
export type WalkAct = "next" | "back" | "replay" | "autoplay_on" | "autoplay_off";

/**
 * Whether the page has a walk, and its state as a JSON string. **Stringified in the page and bounded
 * again here**, so a state of fifty megabytes is a refusal and not a message across the process.
 */
export function readSource(): string {
  return `(() => {
    try {
      const walk = window.__armadaWalk;
      if (walk === undefined || walk === null || typeof walk.state !== "function") return null;
      const said = JSON.stringify(walk.state());
      return typeof said === "string" && said.length <= ${WALK_RAW_MAX} ? said : null;
    } catch {
      return null;
    }
  })()`;
}

/** Tell the walk to leave its card to the bar, once per document. */
export function hostSource(): string {
  return `(() => {
    try {
      window.__armadaWalk.hostCard(true);
    } catch {}
    return null;
  })()`;
}

/** The call for one act. Fixed text per act: nothing the bar or the page sent is spliced in. */
export function actSource(act: WalkAct): string {
  const call: Record<WalkAct, string> = {
    next: "next()",
    back: "back()",
    replay: "replay()",
    autoplay_on: "autoplay(true)",
    autoplay_off: "autoplay(false)",
  };
  return `(() => {
    try {
      window.__armadaWalk.${call[act]};
    } catch {}
    return null;
  })()`;
}

/** A line of the page's words, on one line and no longer than `max`. */
function line(said: string, max: number): string {
  return [...said.replace(/\s+/g, " ").trim()].slice(0, max).join("");
}

/**
 * The walk state the page answered, or `null` for anything that is not one. **The page can answer with
 * anything**: a string past the cap, text that is not JSON, a number out of its range, a field of the
 * wrong type are all refused whole, and a long name or caption is cut. Only the eight known fields
 * survive, and the bar draws them as text. **`stopped` absent is not stopped**, so a page from before it still reads.
 */
export function readWalk(answer: unknown): CaptureWalk | null {
  if (typeof answer !== "string" || answer.length > WALK_RAW_MAX) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(answer);
  } catch {
    return null;
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return null;
  const read = parsed as Record<string, unknown>;
  const { name, said, at, of, autoplay, ready, done, stopped = false } = read;
  if (typeof name !== "string" || typeof said !== "string") return null;
  if (typeof autoplay !== "boolean" || typeof ready !== "boolean" || typeof done !== "boolean" || typeof stopped !== "boolean") return null;
  if (typeof at !== "number" || typeof of !== "number" || !Number.isInteger(at) || !Number.isInteger(of)) return null;
  if (of < 1 || of > WALK_STEPS_MAX || at < 1 || at > of) return null;
  return { name: line(name, NAME_MAX), at, of, said: line(said, SAID_MAX), autoplay, ready, done, stopped };
}

/**
 * The steps of the page's walk it stopped on, told once each until the walk moves off it. **The words are fixed and the
 * numbers main bounded**: the page's name and caption go into the Job's prompt nowhere. Fleet keys its wording on `step `.
 */
export class StoppedSteps {
  /** The step last told, as `at/of`. `null`: none, or the walk has moved since. */
  private told: string | null = null;

  /** One read of the page's walk. What to tell the Job, or `null` for nothing. */
  seen(walk: CaptureWalk | null): string | null {
    if (walk === null || walk.stopped !== true) {
      this.told = null;
      return null;
    }
    const step = `${walk.at}/${walk.of}`;
    if (this.told === step) return null;
    this.told = step;
    return `step ${walk.at} of ${walk.of}: the walk in the page stopped on this step`;
  }
}

/** Whether the ring and caption of the steps Bridge plays draw the same. */
function sameGuide(a: CaptureWalk["guide"], b: CaptureWalk["guide"]): boolean {
  if (a === undefined || b === undefined) return a === b;
  const at = a.box;
  const to = b.box;
  const same = at === null || to === null ? at === to : at.x === to.x && at.y === to.y && at.width === to.width && at.height === to.height;
  return same && a.folded === b.folded && a.stopped === b.stopped && a.signingIn === b.signingIn && a.waiting === b.waiting;
}

/** Whether two states draw the same, so the bar is told only of a change. */
export function sameWalk(a: CaptureWalk | null, b: CaptureWalk | null): boolean {
  if (a === null || b === null) return a === b;
  return (
    a.name === b.name &&
    a.at === b.at &&
    a.of === b.of &&
    a.said === b.said &&
    a.autoplay === b.autoplay &&
    a.ready === b.ready &&
    a.done === b.done &&
    a.stopped === b.stopped &&
    sameGuide(a.guide, b.guide)
  );
}

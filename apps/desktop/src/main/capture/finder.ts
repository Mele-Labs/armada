// What main asks of a page when Bridge plays a walk's steps — `docs/practices/capture-window.md`,
// *A walk in the page*.
//
// **An isolated world, and the page is told nothing.** The finder is a script main evaluates in a
// world of its own (`executeJavaScriptInIsolatedWorld`), which shares the page's DOM and nothing
// else: the page's scripts see no global, no listener and no node of Bridge's, and the world dies
// with the document. It is installed on the first ask of each document and asked again by name, so
// a navigation costs one install, not a standing layer. Every answer is a small plain value that
// main re-bounds here, as `locate.ts`'s is.

import type { WebContents } from "electron";

import type { WalkTarget } from "@armada/protocol";
import type { CaptureRect } from "../../shared/capture-window";
import { rectOf } from "./bounds";
import source from "virtual:armada-walk-finder";

/** The world the finder runs in. Outside Electron's own (999) and the range extensions reserve. */
export const FINDER_WORLD = 2000;

/** Longest one ask waits on the page beyond the patience it was given. */
const SLACK_MS = 2000;

/** What the held element is, for the keys that must reach it. */
export type HeldKind = "field" | "select" | "other";

const whole = (value: unknown): number => (typeof value === "number" && Number.isFinite(value) ? Math.round(value) : 0);

/** A point inside the viewport, re-bounded. */
function pointOf(value: unknown, viewport: { width: number; height: number }): { x: number; y: number } | null {
  if (typeof value !== "object" || value === null) return null;
  const at = value as Record<string, unknown>;
  return {
    x: Math.min(Math.max(whole(at["x"]), 0), viewport.width),
    y: Math.min(Math.max(whole(at["y"]), 0), viewport.height),
  };
}

export class PageFinder {
  private readonly contents: () => WebContents | null;
  private readonly viewport: () => { width: number; height: number };

  /** `contents` is the page, or `null` where it may not be asked: gone, or off the origin it is pinned to. */
  constructor(contents: () => WebContents | null, viewport: () => { width: number; height: number }) {
    this.contents = contents;
    this.viewport = viewport;
  }

  /** Run one expression in the finder's world, installing the finder first where this document has none. */
  private async run(expression: string, patience: number): Promise<unknown> {
    const contents = this.contents();
    if (contents === null || contents.isDestroyed()) return null;
    let giveUp: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        (async () => {
          const installed = await contents.executeJavaScriptInIsolatedWorld(FINDER_WORLD, [
            { code: 'typeof ArmadaFinder === "object"' },
          ]);
          if (installed !== true) await contents.executeJavaScriptInIsolatedWorld(FINDER_WORLD, [{ code: source }]);
          return await contents.executeJavaScriptInIsolatedWorld(FINDER_WORLD, [{ code: expression }]);
        })(),
        new Promise<null>((resolve) => (giveUp = setTimeout(() => resolve(null), patience + SLACK_MS))),
      ]);
    } catch {
      // A page that navigated under the ask, or threw, answers nothing.
      return null;
    } finally {
      clearTimeout(giveUp);
    }
  }

  private seen(answer: unknown): CaptureRect | null {
    if (typeof answer !== "object" || answer === null) return null;
    const box = rectOf(answer, this.viewport());
    return box.width > 0 && box.height > 0 ? box : null;
  }

  /** Look for the target, scroll it into view and wait for it to hold still. The page holds it for what follows. */
  async arrive(target: WalkTarget, patience: number): Promise<CaptureRect | null> {
    return this.seen(await this.run(`ArmadaFinder.finder.arrive(${JSON.stringify(target)}, ${Math.round(patience)})`, patience));
  }

  /** Where the held element is now, or `null` where it left the page. */
  async box(): Promise<CaptureRect | null> {
    return this.seen(await this.run("ArmadaFinder.finder.box()", 0));
  }

  /** Give the held element the keyboard, and say what it is. `null`: it is gone. */
  async focus(): Promise<HeldKind | null> {
    const kind = await this.run("ArmadaFinder.finder.focus()", 0);
    return kind === "field" || kind === "select" || kind === "other" ? kind : null;
  }

  /** Where `words` start and end inside the held element, for a pointer to drag across. */
  async words(words: string): Promise<{ from: { x: number; y: number }; to: { x: number; y: number } } | null> {
    const answer = await this.run(`ArmadaFinder.finder.words(${JSON.stringify(words)})`, 0);
    if (typeof answer !== "object" || answer === null) return null;
    const said = answer as Record<string, unknown>;
    const from = pointOf(said["from"], this.viewport());
    const to = pointOf(said["to"], this.viewport());
    return from === null || to === null ? null : { from, to };
  }

  /** Let the page's finder go of the element. */
  async release(): Promise<void> {
    await this.run("ArmadaFinder.finder.release()", 0);
  }

  /** The page's accessibility tree as text, no longer than `most`. Empty where the page did not answer. */
  async snapshot(most: number): Promise<string> {
    const said = await this.run(`ArmadaFinder.finder.snapshot(${Math.round(most)})`, 5_000);
    return typeof said === "string" ? said.slice(0, most) : "";
  }
}

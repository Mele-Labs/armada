// Where the mock's scenario picker sits, and whether it is collapsed — kept
// across the reload that choosing a scenario causes. Dev-only: nothing the
// Electron build bundles imports this file.
//
// **The mechanism is the one the app already uses for a remembered layout** —
// `guide-list-width.ts` and `packages/shell/src/left-width.ts`, one
// `localStorage` key per thing remembered, every read and every write wrapped.
// A throw here would take the whole mock page down, and this is a dev
// convenience: the honest answer to a storage that refuses is to forget.

/** A spot in the window, as the card's own top-leading corner. */
export type Spot = { x: number; y: number };

/** What a box occupies. `DOMRect` satisfies it, which is where it comes from. */
export type Size = { width: number; height: number };

const SPOT_KEY = "armada.mock.picker-spot";
const COLLAPSED_KEY = "armada.mock.picker-collapsed";

/**
 * The remembered spot, or `null` for one never moved — which is not the same as
 * a spot of `0, 0`, and is why the absent case is a `null` rather than a
 * default: unmoved, the card rests in the corner its stylesheet anchors it to.
 * A value that is not two finite numbers is read as absent, so a hand-edited or
 * half-written entry forgets rather than throws.
 */
export function readSpot(): Spot | null {
  try {
    const raw = window.localStorage.getItem(SPOT_KEY);
    if (raw === null) return null;
    const held: unknown = JSON.parse(raw);
    if (typeof held !== "object" || held === null) return null;
    const { x, y } = held as { x?: unknown; y?: unknown };
    if (typeof x !== "number" || typeof y !== "number") return null;
    if (!Number.isFinite(x) || !Number.isFinite(y)) return null;
    return { x, y };
  } catch {
    return null;
  }
}

export function writeSpot(spot: Spot): void {
  try {
    window.localStorage.setItem(SPOT_KEY, JSON.stringify(spot));
  } catch {
    // Unremembered is the honest answer for a dev preference.
  }
}

/** Back to the corner the stylesheet rests it in, with nothing remembered. */
export function forgetSpot(): void {
  try {
    window.localStorage.removeItem(SPOT_KEY);
  } catch {
    // As above.
  }
}

export function readCollapsed(): boolean {
  try {
    return window.localStorage.getItem(COLLAPSED_KEY) === "true";
  } catch {
    return false;
  }
}

export function writeCollapsed(collapsed: boolean): void {
  try {
    window.localStorage.setItem(COLLAPSED_KEY, String(collapsed));
  } catch {
    // As above.
  }
}

/** The window the card is being placed in. Its own argument so a test can hand a smaller one. */
function theWindow(): Size {
  return { width: window.innerWidth, height: window.innerHeight };
}

/**
 * `spot` with the whole card inside `within`.
 *
 * **Every edge, not only the one being dragged towards.** A remembered spot is
 * read back in whatever window is open now — a narrower one, or the same window
 * with the card expanded since — so the recovery and the drag limit are one
 * function rather than a clamp on the way out and a hope on the way in.
 *
 * A card larger than the window it is in has no spot that satisfies both edges;
 * it goes to the leading one, where its own handle is, because that is the half
 * a person needs in order to move it again.
 */
export function clampSpot(spot: Spot, size: Size, within: Size = theWindow()): Spot {
  return {
    x: Math.min(Math.max(0, within.width - size.width), Math.max(0, spot.x)),
    y: Math.min(Math.max(0, within.height - size.height), Math.max(0, spot.y)),
  };
}

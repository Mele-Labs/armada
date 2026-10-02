import { useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

/**
 * A panel's inner-edge resize handle, and the range it drags in. **One
 * handle for every panel that resizes**: Helm's dock (`TheShell`) and every
 * `Sheet` (the owner's notes of 30 Sep and 2 Oct 2026 — "resize it with the
 * resize handle we have", "every sheet should be resizable") draw this one,
 * so the edges look and answer alike.
 *
 * A drag or an arrow key moves it; both read the same clamp so neither can
 * push the dock past what a mouse could reach.
 *
 * **Left widens the dock, right narrows it** — a dock sits on its container's
 * trailing edge, so dragging toward the content is dragging the edge that
 * grows it, the same direction a mouse drag moves. Home and End match: Home
 * (the leftmost position a splitter can take) is the widest the dock gets.
 */

// Fallbacks only for a caller with no stylesheet loaded (a bare unit test);
// the tokens are the real source and are read fresh on every drag. The max
// fallback stands in for a whole computed ceiling, not one token, since a
// caller with no stylesheet has no width figure worth trusting either.
const DOCK_WIDTH_MIN_FALLBACK = 320;
const DOCK_WIDTH_MAX_FALLBACK = 640;
const DOCK_WIDTH_DEFAULT_FALLBACK = 380;

/** One `--space-4` per arrow press — the same step the dock's own padding uses. */
export const DOCK_WIDTH_STEP = 16;

/** A length token in px, read off the root; `NaN` with no document or no stylesheet. */
export function tokenPx(name: string): number {
  if (typeof document === "undefined") return Number.NaN;
  return parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
}

/**
 * A dock's drag range, given `room` — the width the dock and the content under
 * it share once the chrome around them is taken off. The floor is
 * `--w-dock-min`, which keeps a dock's head row from clipping. **The ceiling
 * has no token**: it is `room` minus `--w-work-min`, what always stays
 * uncovered so a dock dragged wide never leaves the content a sliver.
 */
export function dockWidthRange(room: number): { min: number; max: number } {
  const min = tokenPx("--w-dock-min");
  const floor = Number.isFinite(min) ? min : DOCK_WIDTH_MIN_FALLBACK;
  const workMin = tokenPx("--w-work-min");
  if (!Number.isFinite(room) || !Number.isFinite(workMin)) {
    return { min: floor, max: DOCK_WIDTH_MAX_FALLBACK };
  }
  return { min: floor, max: Math.max(floor, room - workMin) };
}

export function clampToRange(width: number, { min, max }: { min: number; max: number }): number {
  return Math.min(max, Math.max(min, width));
}

/** A dock's own resting width, in px, for a caller with none of its own to remember yet. */
export function defaultDockWidth(): number {
  const value = tokenPx("--w-dock");
  return Number.isFinite(value) ? value : DOCK_WIDTH_DEFAULT_FALLBACK;
}

export type DockHandleProps = {
  /** The dock's width as drawn — already clamped, so the drag anchors on what is on screen. */
  width: number;
  /** The range, from `dockWidthRange`. */
  min: number;
  max: number;
  /** What it resizes, for the separator's name — `Resize Helm`. */
  label: string;
  /**
   * The way a drag widens it. `left` for a panel on its container's trailing
   * edge, the dock's; `right` for one on the leading edge, a left sheet's.
   */
  grows?: "left" | "right";
  onResize: (width: number) => void;
};

export function DockHandle({ width, min, max, label, grows = "left", onResize }: DockHandleProps) {
  const drag = useRef<{ pointerId: number; startX: number; startWidth: number } | null>(null);
  // Only for the line's own intensified colour while dragging — `:hover` drops
  // the moment the cursor leaves the 8px hit area, which a fast drag does
  // almost at once, and the grip going dim mid-drag would read as let go.
  const [dragging, setDragging] = useState(false);
  const range = { min, max };
  const sign = grows === "left" ? 1 : -1;

  function pointerDown(event: PointerEvent<HTMLDivElement>): void {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, startX: event.clientX, startWidth: width };
    setDragging(true);
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    // Suppressing the drag's own text selection also suppresses the focus a
    // click would otherwise grant — put back by hand, so the keyboard still
    // works right after a press finds the handle.
    event.currentTarget.focus();
    event.preventDefault();
  }

  function pointerMove(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current === null || drag.current.pointerId !== event.pointerId) return;
    const delta = sign * (drag.current.startX - event.clientX);
    onResize(clampToRange(drag.current.startWidth + delta, range));
  }

  function endDrag(event: PointerEvent<HTMLDivElement>): void {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
  }

  function keyDown(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "ArrowLeft") onResize(clampToRange(width + sign * DOCK_WIDTH_STEP, range));
    else if (event.key === "ArrowRight") onResize(clampToRange(width - sign * DOCK_WIDTH_STEP, range));
    else if (event.key === "Home") onResize(sign > 0 ? max : min);
    else if (event.key === "End") onResize(sign > 0 ? min : max);
    else return;
    event.preventDefault();
  }

  return (
    <div
      className="armada-dock-handle"
      role="separator"
      aria-orientation="vertical"
      aria-label={`Resize ${label}`}
      aria-valuenow={Math.round(width)}
      aria-valuemin={Math.round(min)}
      aria-valuemax={Math.round(max)}
      data-dragging={dragging || undefined}
      tabIndex={0}
      onPointerDown={pointerDown}
      onPointerMove={pointerMove}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onKeyDown={keyDown}
    />
  );
}

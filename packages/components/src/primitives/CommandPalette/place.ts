// Where the palette sits: where he last dragged it, else over the title bar's field it opened from, else the
// contract's top-anchored default. He moves it by any part of it that is not a field, a row or a button, and it snaps to
// the window's middle, across and down, when its own middle comes near (the owner, 27 Oct 2026). Kept on this machine.

import { useCallback, useLayoutEffect, useState, type CSSProperties, type PointerEvent as ReactPointerEvent, type RefObject } from "react";

/** The palette's middle, as a share of the window's width, and its top, as a share of its height: a resize keeps both. */
type Placed = { x: number; y: number };

/** Where he left it, in this machine's storage, so it opens there the next time and after a restart. */
const KEPT = "armada.palette.place";

/** How near the middle, in pixels, before it snaps there. */
const SNAP = 12;

/** What a press may start a drag on: anything the palette draws but a field, a row, a link or a button. */
const HANDS_OFF = 'input, textarea, button, a, [role="option"]';

function kept(): Placed | null {
  try {
    const read = JSON.parse(window.localStorage.getItem(KEPT) ?? "null") as Placed | null;
    return read !== null && typeof read.x === "number" && typeof read.y === "number" ? read : null;
  } catch {
    return null;
  }
}

/** The middle lines the palette is snapped to while it moves, for the guides drawn across the window. */
export type Snapped = { across: boolean; down: boolean };

export function usePlace(
  open: boolean,
  anchor: string | undefined,
  panel: RefObject<HTMLDivElement | null>,
): {
  style: CSSProperties | undefined;
  snapped: Snapped | null;
  onPointerDown: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onDoubleClick: (event: React.MouseEvent<HTMLDivElement>) => void;
} {
  const [placed, setPlaced] = useState<Placed | null>(kept);
  const [field, setField] = useState<{ x: number; top: number } | null>(null);
  const [moving, setMoving] = useState<{ x: number; top: number; snapped: Snapped } | null>(null);

  // The field it opened over, read once on opening, as `Popover` reads its trigger: the palette's middle lands on the
  // field's middle, and its input on the field's top.
  useLayoutEffect(() => {
    if (!open || anchor === undefined) return setField(null);
    const box = document.querySelector(anchor)?.getBoundingClientRect();
    if (box === undefined || box.width === 0) return setField(null);
    setField({ x: box.left + box.width / 2, top: box.top });
  }, [open, anchor]);

  const onPointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (event.button !== 0 || (event.target as Element).closest(HANDS_OFF) !== null) return;
      const box = panel.current?.getBoundingClientRect();
      if (box === undefined) return;
      event.preventDefault();
      const from = { x: event.clientX, y: event.clientY, middle: box.left + box.width / 2, top: box.top, height: box.height };
      let last: { x: number; top: number } | null = null;
      function move(at: PointerEvent) {
        let x = from.middle + at.clientX - from.x;
        let top = from.top + at.clientY - from.y;
        const across = Math.abs(x - window.innerWidth / 2) < SNAP;
        const down = Math.abs(top + from.height / 2 - window.innerHeight / 2) < SNAP;
        if (across) x = window.innerWidth / 2;
        if (down) top = window.innerHeight / 2 - from.height / 2;
        // Never off the window: the field stays reachable.
        top = Math.min(Math.max(top, 0), window.innerHeight - from.height / 4);
        last = { x, top };
        setMoving({ x, top, snapped: { across, down } });
      }
      function done() {
        window.removeEventListener("pointermove", move);
        window.removeEventListener("pointerup", done);
        setMoving(null);
        if (last === null) return;
        const next = { x: last.x / window.innerWidth, y: last.top / window.innerHeight };
        window.localStorage.setItem(KEPT, JSON.stringify(next));
        setPlaced(next);
      }
      window.addEventListener("pointermove", move);
      window.addEventListener("pointerup", done);
    },
    [panel],
  );

  // A double click on the same ground puts it back where it opens by default.
  const onDoubleClick = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if ((event.target as Element).closest(HANDS_OFF) !== null) return;
    window.localStorage.removeItem(KEPT);
    setPlaced(null);
  }, []);

  const at = moving ?? (placed === null ? field : { x: placed.x * window.innerWidth, top: placed.y * window.innerHeight });
  return {
    style: at === null ? undefined : { left: at.x, top: at.top },
    snapped: moving?.snapped ?? null,
    onPointerDown,
    onDoubleClick,
  };
}

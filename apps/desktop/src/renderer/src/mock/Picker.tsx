// Which scenario the mock is on, and a way to another. Dev-only: nothing the
// Electron build bundles imports this file.

import { StrictMode, useLayoutEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Button, Card, Select } from "@armada/components";
import type { KeyboardEvent, PointerEvent } from "react";

import "./mock.css";
import { SCENARIOS } from "./scenario";
import type { Scenario } from "./scenario";
import {
  clampSpot,
  forgetSpot,
  readCollapsed,
  readSpot,
  writeCollapsed,
  writeSpot,
} from "./picker-place";
import type { Spot } from "./picker-place";

/**
 * The scenarios grouped by what comes before the first `/`, in the order
 * `SCENARIOS` lists them. **A name with no `/` is its own first group** — the
 * whole-app moments are what the mock opens on, and burying them under a
 * heading would put `every-state` below a dozen `arc/…` rows.
 */
function grouped(): [string, Scenario[]][] {
  const groups = new Map<string, Scenario[]>();
  for (const one of SCENARIOS) {
    const at = one.name.indexOf("/");
    const key = at === -1 ? "" : one.name.slice(0, at);
    const held = groups.get(key);
    if (held === undefined) groups.set(key, [one]);
    else held.push(one);
  }
  return [...groups.entries()];
}

/** One `--space-4` per arrow press, the step `TheShell`'s own handle takes. */
const STEP = 16;

/**
 * A drag underway: which pointer, where it went down, the card's box then, and
 * where the card has reached. **`moved` is why the spot is not read out of state
 * on the lift**: a `pointermove` is a continuous event, so React is free not to
 * have committed the last one when the discrete `pointerup` runs.
 */
type Drag = {
  pointerId: number;
  fromX: number;
  fromY: number;
  at: Spot;
  size: DOMRect;
  moved: Spot | null;
};

/**
 * What the grip does, said for a keyboard: a cursor over it is not a label. The
 * grip draws no text at all — it is a bar, and the scenario is the select's to
 * say — so the name is on the end of the label, since a name a speech command
 * cannot say is a control it cannot press.
 */
const moveLabel = (current: string) => `Move the scenario picker — ${current}`;
const MOVE_HINT = "Drag, or nudge with the arrow keys. Home returns it to the corner.";

/**
 * Collapsed, the whole chip is the press that reopens it — so its label says
 * what pressing does rather than leaving a screen reader the bare scenario name
 * the chip draws. It is still the grip, which the hint carries.
 */
const expandLabel = (current: string) => `Expand the scenario picker — ${current}`;
const CHIP_HINT = `Press to expand. ${MOVE_HINT}`;

/**
 * Choosing reloads on `?scenario=`, so a scenario never inherits the last one's window state.
 *
 * **The picker is moved rather than parked.** It rested over Helm's column on
 * the assumption that column is empty below its composer, and on the screens
 * being annotated it is not — so the spot and the collapse are the owner's, kept
 * across that reload. The drag follows `LeftHandle` in `packages/components`:
 * pointer capture on the handle, a clamp, the same keys without a pointer. It
 * is a move, so the clamp bounds two axes and the handle is a real button —
 * there is no role for "drag me", and a keyboard has to reach it.
 *
 * **Expanded it is one row saying the scenario once**, and **collapsed it is a
 * chip rather than a smaller card**: one small control tall, the name, and the
 * whole of it presses to reopen.
 */
export function Picker({ current }: { current: string }) {
  const frame = useRef<HTMLDivElement>(null);
  const [spot, setSpot] = useState<Spot | null>(() => readSpot());
  const [collapsed, setCollapsed] = useState(() => readCollapsed());
  const [dragging, setDragging] = useState(false);
  const drag = useRef<Drag | null>(null);
  // A drag of the chip ends in a `click` on it, since the chip is both grip and
  // press. Moved, that press is not one, or every lift would reopen the picker.
  const shifted = useRef(false);

  /** Settled: held in state, and remembered for the next page. */
  function place(next: Spot): void {
    setSpot(next);
    writeSpot(next);
  }

  /** The picker's box now — what a clamp needs, and where a first drag starts from. */
  function box(): DOMRect | null {
    return frame.current?.getBoundingClientRect() ?? null;
  }

  // A spot is read back into whatever window is open now, and collapsing
  // changes the picker's own size — one recovery, so both run through the clamp
  // here. `place` rather than `setSpot`: an unremembered recovery would read
  // the offscreen spot back on the next reload.
  useLayoutEffect(() => {
    if (spot === null) return;
    function settle(): void {
      const size = box();
      if (size === null) return;
      const fixed = clampSpot(spot!, size);
      if (fixed.x !== spot!.x || fixed.y !== spot!.y) place(fixed);
    }
    settle();
    window.addEventListener("resize", settle);
    return () => window.removeEventListener("resize", settle);
  }, [spot, collapsed]);

  function pointerDown(event: PointerEvent<HTMLButtonElement>): void {
    if (event.button !== 0) return;
    const size = box();
    if (size === null) return;
    // Capture keeps the moves coming to the grip when the pointer outruns it.
    // A pointer the browser is not tracking has none to give, and a drag driven
    // by hand is exactly that — the moves still arrive, so it is not a failure.
    try {
      event.currentTarget.setPointerCapture(event.pointerId);
    } catch {
      // As above.
    }
    shifted.current = false;
    drag.current = {
      pointerId: event.pointerId,
      fromX: event.clientX,
      fromY: event.clientY,
      at: { x: size.left, y: size.top },
      size,
      moved: null,
    };
    setDragging(true);
    document.body.style.cursor = "grabbing";
    document.body.style.userSelect = "none";
    // `preventDefault` stops the press selecting text, and takes the focus with it.
    event.currentTarget.focus();
    event.preventDefault();
  }

  function pointerMove(event: PointerEvent<HTMLButtonElement>): void {
    const held = drag.current;
    if (held === null || held.pointerId !== event.pointerId) return;
    const next = clampSpot(
      { x: held.at.x + (event.clientX - held.fromX), y: held.at.y + (event.clientY - held.fromY) },
      held.size,
    );
    // State only: what gets remembered is where he left it, not every frame of getting there.
    held.moved = next;
    shifted.current = true;
    setSpot(next);
  }

  function endDrag(event: PointerEvent<HTMLButtonElement>): void {
    const held = drag.current;
    if (held === null || held.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    if (held.moved !== null) writeSpot(held.moved);
  }

  function keyDown(event: KeyboardEvent<HTMLButtonElement>): void {
    const size = box();
    if (size === null) return;
    // Unmoved, the first press starts from wherever the stylesheet rests it.
    const at = spot ?? { x: size.left, y: size.top };
    if (event.key === "ArrowLeft") place(clampSpot({ ...at, x: at.x - STEP }, size));
    else if (event.key === "ArrowRight") place(clampSpot({ ...at, x: at.x + STEP }, size));
    else if (event.key === "ArrowUp") place(clampSpot({ ...at, y: at.y - STEP }, size));
    else if (event.key === "ArrowDown") place(clampSpot({ ...at, y: at.y + STEP }, size));
    else if (event.key === "Home") {
      // The one way back to the resting corner, for a card left somewhere awkward.
      setSpot(null);
      forgetSpot();
    } else return;
    event.preventDefault();
  }

  function toggle(): void {
    setCollapsed((held) => {
      writeCollapsed(!held);
      return !held;
    });
  }

  /** The chip pressed, which is the chip not dragged. */
  function press(): void {
    if (shifted.current) {
      shifted.current = false;
      return;
    }
    toggle();
  }

  const grip = {
    onPointerDown: pointerDown,
    onPointerMove: pointerMove,
    onPointerUp: endDrag,
    onPointerCancel: endDrag,
    onKeyDown: keyDown,
  };

  return (
    // The spot is on a frame around the picker rather than on what it draws:
    // `Card` takes no ref, and the box being measured is the one being moved.
    <div
      ref={frame}
      className="armada-mock-picker"
      data-placed={spot === null ? undefined : true}
      data-collapsed={collapsed || undefined}
      data-dragging={dragging || undefined}
      // React writes a number on `left`/`top` as px itself, so no length is
      // spelled — and none of these could be a token: the value is a pointer's.
      style={spot === null ? undefined : { left: spot.x, top: spot.y }}
    >
      {collapsed ? (
        // The chip is the grip and the press at once, so it carries both.
        <Button
          variant="secondary"
          size="sm"
          data-chip
          aria-label={expandLabel(current)}
          aria-expanded={false}
          title={CHIP_HINT}
          onClick={press}
          {...grip}
        >
          {current}
        </Button>
      ) : (
        <Card>
          <div className="armada-mock-picker__bar">
            <Button
              variant="ghost"
              size="sm"
              // A data attribute rather than a class: `Button` writes its own
              // `className` after the spread, so one handed to it is dropped.
              data-grip
              aria-label={moveLabel(current)}
              title={MOVE_HINT}
              {...grip}
            />
            <div className="armada-mock-picker__field">
              <Select
                // Named for a screen reader, drawn for nobody: a label over the
                // one control on a dev tool was the third element saying this.
                aria-label="Mock scenario"
                value={current}
                onChange={(event) => {
                  const url = new URL(window.location.href);
                  url.searchParams.set("scenario", event.target.value);
                  window.location.assign(url);
                }}
              >
                {grouped().map(([group, scenarios]) =>
                  group === "" ? (
                    scenarios.map((one) => (
                      <option key={one.name} value={one.name} title={one.says}>
                        {one.name}
                      </option>
                    ))
                  ) : (
                    <optgroup key={group} label={group}>
                      {scenarios.map((one) => (
                        <option key={one.name} value={one.name} title={one.says}>
                          {one.name}
                        </option>
                      ))}
                    </optgroup>
                  ),
                )}
              </Select>
            </div>
            <Button variant="ghost" size="sm" aria-expanded onClick={toggle}>
              Minimize
            </Button>
          </div>
        </Card>
      )}
    </div>
  );
}

/**
 * The picker on its own root, in `host` — the mock's own mount, so the page and
 * a test put up the same card with the same stylesheet behind it.
 */
export function mountPicker(current: string, host: HTMLElement): () => void {
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Picker current={current} />
    </StrictMode>,
  );
  return () => root.unmount();
}

// A walk played over the app, on `?walk=<name>`: each step's target ringed,
// its caption in a card, and Next to move on. Dev-only: nothing the Electron
// build bundles imports this file. `walk.ts` is the engine, shared with the
// walk's test and the capture command.

import { StrictMode, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import { createRoot } from "react-dom/client";
import { Alert, Button, Card, Switch } from "@armada/components";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";

import "./mock.css";
import { WALK_UI, act, arrive, stopped } from "./walk";
import { keepInside, placeCard } from "./walk-card";
import type { Walk as WalkScript } from "./walk";

/** How long autoplay holds each step before it moves on. */
const DWELL_MS = 3_000;

/**
 * Where the walk is. `seeking` waits for the step's target, `ready` rings it,
 * `stopped` is a target that never came, and `done` is past the last step.
 */
type Phase =
  | { is: "seeking"; at: number }
  | { is: "ready"; at: number; element: HTMLElement }
  | { is: "stopped"; at: number }
  | { is: "done" };

/** The target's box, followed every frame: a panel slides, and a list scrolls under it. */
function useBox(element: HTMLElement | null): DOMRect | null {
  const [box, setBox] = useState<DOMRect | null>(null);
  useEffect(() => {
    if (element === null) {
      setBox(null);
      return;
    }
    let frame = 0;
    const follow = () => {
      const now = element.getBoundingClientRect();
      setBox((was) =>
        was !== null && was.x === now.x && was.y === now.y && was.width === now.width && was.height === now.height
          ? was
          : now,
      );
      frame = requestAnimationFrame(follow);
    };
    follow();
    return () => cancelAnimationFrame(frame);
  }, [element]);
  return box;
}

/** Where the card keeps from the window's edge, and how far down it starts under the top bar: `mock.css`'s `--space-6` and `--space-12 + --space-3`. */
const GAP = 24;
const TOP = 60;

/** The spot a person dragged the card to, kept for the session. */
const SPOT = "armada.mock.walk-card";
const savedSpot = (): { x: number; y: number } | null => {
  try {
    const raw = window.sessionStorage.getItem(SPOT);
    return raw === null ? null : (JSON.parse(raw) as { x: number; y: number });
  } catch {
    return null;
  }
};
const saveSpot = (spot: { x: number; y: number }) => {
  try {
    window.sessionStorage.setItem(SPOT, JSON.stringify(spot));
  } catch {
    // A window that keeps nothing keeps nothing; the card is where it was dragged until it reloads.
  }
};

/** Which edge the card rests on, from where the ring is: `placeCard`. */
function useEdge(card: HTMLElement | null, ring: DOMRect | null, placed: boolean, folded: boolean): "bottom" | "top" {
  const [edge, setEdge] = useState<"bottom" | "top">("bottom");
  useLayoutEffect(() => {
    if (card === null || placed) return;
    const { width, height } = card.getBoundingClientRect();
    setEdge(placeCard(ring, { width, height }, { width: window.innerWidth, height: window.innerHeight }, GAP, TOP));
  }, [card, ring, placed, folded]);
  return edge;
}

/** Whether a key is going into something a person types in, which `.` must leave alone. */
const typing = (target: EventTarget | null) =>
  target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

export function WalkPlayer({ name, script, autoplay: startPlaying }: { name: string; script: WalkScript; autoplay: boolean }) {
  const [phase, setPhase] = useState<Phase>({ is: "seeking", at: 0 });
  const [autoplay, setAutoplay] = useState(startPlaying);
  const [card, setCard] = useState<HTMLDivElement | null>(null);
  const box = useBox(phase.is === "ready" ? phase.element : null);
  const [spot, setSpot] = useState(savedSpot);
  const [folded, setFolded] = useState(false);
  const edge = useEdge(card, box, spot !== null, folded);
  const steps = script.steps;

  const at = phase.is === "done" ? steps.length : phase.at;
  useEffect(() => {
    if (phase.is !== "seeking") return;
    let gone = false;
    void arrive(steps[at]!).then((element) => {
      if (gone) return;
      setPhase(element === null ? { is: "stopped", at } : { is: "ready", at, element });
    });
    return () => {
      gone = true;
    };
  }, [phase.is, at, steps]);

  const next = useCallback(() => {
    if (phase.is !== "ready") return;
    act(steps[phase.at]!, phase.element);
    setPhase(phase.at + 1 < steps.length ? { is: "seeking", at: phase.at + 1 } : { is: "done" });
  }, [phase, steps]);

  // `.` folds the card to a pill and opens it again, except where a field has the keys.
  useEffect(() => {
    const fold = (event: KeyboardEvent) => {
      if (event.key === "." && !event.metaKey && !event.ctrlKey && !event.altKey && !typing(event.target)) setFolded((was) => !was);
    };
    window.addEventListener("keydown", fold);
    return () => window.removeEventListener("keydown", fold);
  }, []);

  /** Drags the card by its header, and keeps the spot it is dropped on. */
  const drag = (event: ReactPointerEvent<HTMLElement>) => {
    if (card === null || event.button !== 0 || (event.target as HTMLElement).closest("button") !== null) return;
    const here = card.getBoundingClientRect();
    const grab = { x: event.clientX - here.left, y: event.clientY - here.top };
    const size = { width: here.width, height: here.height };
    const view = () => ({ width: window.innerWidth, height: window.innerHeight });
    const move = (to: PointerEvent) => setSpot(keepInside({ x: to.clientX - grab.x, y: to.clientY - grab.y }, size, view()));
    const drop = (to: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", drop);
      saveSpot(keepInside({ x: to.clientX - grab.x, y: to.clientY - grab.y }, size, view()));
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", drop);
  };

  const later = useRef(0);
  useEffect(() => {
    if (!autoplay || phase.is !== "ready") return;
    later.current = window.setTimeout(next, DWELL_MS);
    return () => window.clearTimeout(later.current);
  }, [autoplay, phase, next]);

  const ring =
    box === null
      ? undefined
      : ({
          "--walk-x": `${box.x}px`,
          "--walk-y": `${box.y}px`,
          "--walk-w": `${box.width}px`,
          "--walk-h": `${box.height}px`,
        } as CSSProperties);

  return (
    // **A press on the card is not a click outside the app's layer.** The
    // palette closes on a mousedown anywhere outside it, so Next closed the
    // palette the step before had opened. Stopped at this root, it never
    // reaches the window the palette listens on.
    <div {...{ [WALK_UI]: "" }} className="armada-mock-walk" onMouseDown={(event) => {
        event.stopPropagation();
        // Nor does it take focus: a field the step before opened is still the one the next step types into.
        event.preventDefault();
      }}>
      {ring !== undefined && <div className="armada-mock-walk__ring" style={ring} />}
      <div
        ref={setCard}
        className="armada-mock-walk__card"
        data-edge={edge}
        data-folded={folded || undefined}
        style={spot === null ? undefined : { left: spot.x, top: spot.y, right: "auto", bottom: "auto" }}
        data-walk-state={phase.is}
        data-walk-step={at + 1}
      >
        {phase.is === "done" ? (
          <Button size="sm" onClick={() => window.location.reload()}>
            Replay {name}
          </Button>
        ) : folded ? (
          <div className="armada-mock-walk__pill" onPointerDown={drag}>
            <span className="armada-mock-walk__where">
              Step {at + 1} of {steps.length}
            </span>
            <Button variant="ghost" size="sm" iconOnly aria-label="Unfold the card" onClick={() => setFolded(false)}>
              <ChevronUp size={14} aria-hidden="true" />
            </Button>
            <Button variant="primary" size="sm" disabled={phase.is !== "ready"} onClick={next}>
              Next
            </Button>
          </div>
        ) : (
          <Card>
            <div className="armada-mock-walk__head" onPointerDown={drag}>
              <p className="armada-mock-walk__where">
                {name} · Step {at + 1} of {steps.length}
              </p>
              <Button variant="ghost" size="sm" iconOnly aria-label="Fold the card" onClick={() => setFolded(true)}>
                <ChevronDown size={14} aria-hidden="true" />
              </Button>
            </div>
            {phase.is === "stopped" ? (
              <div className="armada-mock-walk__said" data-stopped>
                <Alert>{stopped(phase.at, steps[phase.at]!)}</Alert>
              </div>
            ) : (
              <p className="armada-mock-walk__said">{steps[at]!.say}</p>
            )}
            {phase.is !== "stopped" && (
              <div className="armada-mock-walk__bar">
                <Switch checked={autoplay} onChange={(event) => setAutoplay(event.target.checked)}>
                  Autoplay
                </Switch>
                <Button variant="primary" size="sm" disabled={phase.is !== "ready"} onClick={next}>
                  Next
                </Button>
              </div>
            )}
          </Card>
        )}
      </div>
    </div>
  );
}

/** The walk's card and ring, in their own root over the app's. */
export function mountWalk(name: string, script: WalkScript, autoplay: boolean, host: HTMLElement): void {
  createRoot(host).render(
    <StrictMode>
      <WalkPlayer name={name} script={script} autoplay={autoplay} />
    </StrictMode>,
  );
}

/** What the page draws for a walk it has no file for. */
export function mountNoWalk(name: string, host: HTMLElement): void {
  createRoot(host).render(
    <div {...{ [WALK_UI]: "" }} className="armada-mock-walk">
      <div className="armada-mock-walk__card" data-edge="bottom" data-walk-state="stopped">
        <Card>
          <div className="armada-mock-walk__said" data-stopped>
            <Alert>No walk named {name}.</Alert>
          </div>
        </Card>
      </div>
    </div>,
  );
}

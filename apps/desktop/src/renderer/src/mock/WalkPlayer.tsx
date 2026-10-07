// A walk played over the app, on `?walk=<name>`: each step's target ringed,
// its caption in a card, and Next to move on. Dev-only: nothing the Electron
// build bundles imports this file. `walk.ts` is the engine, shared with the
// walk's test and the capture command.

import { StrictMode, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { Alert, Button, Card, Switch } from "@armada/components";
import type { CSSProperties } from "react";

import "./mock.css";
import { WALK_UI, act, arrive, stopped } from "./walk";
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

const overlaps = (a: DOMRect, b: { left: number; right: number; top: number; bottom: number }) =>
  a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;

/**
 * The card rests in the trailing bottom corner, where the scenario picker does,
 * and **crosses to the leading one when the ring is under it**, so it never
 * covers what it is pointing at.
 */
function useSide(card: HTMLElement | null, ring: DOMRect | null): "trailing" | "leading" {
  const [side, setSide] = useState<"trailing" | "leading">("trailing");
  useLayoutEffect(() => {
    if (card === null || ring === null) return;
    const here = card.getBoundingClientRect();
    const there = { left: window.innerWidth - here.right, right: window.innerWidth - here.left, top: here.top, bottom: here.bottom };
    if (overlaps(ring, here) && !overlaps(ring, there)) setSide((was) => (was === "trailing" ? "leading" : "trailing"));
  }, [card, ring]);
  return side;
}

export function WalkPlayer({ name, script, autoplay: startPlaying }: { name: string; script: WalkScript; autoplay: boolean }) {
  const [phase, setPhase] = useState<Phase>({ is: "seeking", at: 0 });
  const [autoplay, setAutoplay] = useState(startPlaying);
  const [card, setCard] = useState<HTMLDivElement | null>(null);
  const box = useBox(phase.is === "ready" ? phase.element : null);
  const side = useSide(card, box);
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
        data-side={side}
        data-walk-state={phase.is}
        data-walk-step={at + 1}
      >
        {phase.is === "done" ? (
          <Button size="sm" onClick={() => window.location.reload()}>
            Replay {name}
          </Button>
        ) : (
          <Card>
            <p className="armada-mock-walk__where">
              {name} · Step {at + 1} of {steps.length}
            </p>
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
      <div className="armada-mock-walk__card" data-side="trailing" data-walk-state="stopped">
        <Card>
          <div className="armada-mock-walk__said" data-stopped>
            <Alert>No walk named {name}.</Alert>
          </div>
        </Card>
      </div>
    </div>,
  );
}

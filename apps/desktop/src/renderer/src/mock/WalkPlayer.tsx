// A walk played over the app, on `?walk=<name>`: each step's target ringed,
// its caption in a card, and Next to move on. Dev-only: nothing the Electron
// build bundles imports this file. `walk.ts` is the engine, shared with the
// walk's test and the capture command.
//
// **Keys**: → or Enter Next, ← Back, A Autoplay, R replay from step 1, . fold.
// **Back is a replay.** A step acts on the app, and nothing undoes that, so Back
// loads the page again on `?step=<n>`: steps before n run straight through, with
// no dwell, and the walk stands ringed on step n.
//
// **`window.__armadaWalk`** is how the walk window's bar reaches the walk in its
// page, and hides the card there. The real window would reach it by main running
// script in the page, as it does for capture.

import { StrictMode, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { ChevronDown, ChevronUp, RotateCcw } from "lucide-react";
import { createRoot } from "react-dom/client";
import { Alert, Button, Card, Kbd, Switch, Tooltip, WalkPressMark, type WalkPress } from "@armada/components";
import type { WalkHandle, WalkState } from "@armada/screens/src/fixtures/walk-window";
import type { CSSProperties, PointerEvent as ReactPointerEvent } from "react";

import "./mock.css";
import { WALK_UI, act, arrive, find, stopped, targetOf } from "./walk";
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

/** Frames the box must hold still before it is no longer looked at. A slide settles inside this. */
const STILL_FRAMES = 30;

/**
 * The target's box. It is looked at every frame while it may be moving (a panel slides, a list scrolls under it) and
 * not at all once it has held still for `STILL_FRAMES`, until something that can move it happens: a scroll, a resize,
 * a transition or animation starting, or the target or the page changing size. A window left open drew a frame
 * and a `getBoundingClientRect` each, for as long as it stood (358 in 10.7 s in the owner's profile).
 */
function useBox(element: HTMLElement | null): DOMRect | null {
  const [box, setBox] = useState<DOMRect | null>(null);
  useEffect(() => {
    if (element === null) {
      setBox(null);
      return;
    }
    let frame = 0;
    let held = 0;
    let last: DOMRect | null = null;
    const follow = () => {
      frame = 0;
      const now = element.getBoundingClientRect();
      const moved = last === null || last.x !== now.x || last.y !== now.y || last.width !== now.width || last.height !== now.height;
      if (moved) {
        last = now;
        setBox(now);
      }
      held = moved ? 0 : held + 1;
      if (held < STILL_FRAMES && !document.hidden) frame = requestAnimationFrame(follow);
    };
    const wake = () => {
      held = 0;
      if (frame === 0 && !document.hidden) frame = requestAnimationFrame(follow);
    };
    wake();
    const resized = new ResizeObserver(wake);
    resized.observe(element);
    resized.observe(document.body);
    const events = ["scroll", "resize", "transitionrun", "animationstart"] as const;
    for (const one of events) window.addEventListener(one, wake, { capture: true, passive: true });
    document.addEventListener("visibilitychange", wake);
    return () => {
      cancelAnimationFrame(frame);
      resized.disconnect();
      for (const one of events) window.removeEventListener(one, wake, { capture: true });
      document.removeEventListener("visibilitychange", wake);
    };
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

/** The walk window's stand-in frames this page, and draws the controls itself. */
const framed = () => window.parent !== window && new URLSearchParams(window.location.search).has("walked");

/** How long the cursor takes to arrive before a press, as the bar's walk waits in `stepwalk.ts`. */
const GLIDE_MS = 300;

export function WalkPlayer({ name, script, autoplay: startPlaying, from = 0 }: { name: string; script: WalkScript; autoplay: boolean; from?: number }) {
  const [phase, setPhase] = useState<Phase>({ is: "seeking", at: 0 });
  // The bar of a window that frames this page draws the controls: the card steps aside, the ring stays.
  const [hosted, setHosted] = useState(framed);
  const [autoplay, setAutoplay] = useState(startPlaying);
  const [card, setCard] = useState<HTMLDivElement | null>(null);
  const box = useBox(phase.is === "ready" ? phase.element : null);
  const [spot, setSpot] = useState(savedSpot);
  const [folded, setFolded] = useState(false);
  const edge = useEdge(card, box, spot !== null, folded);
  // The last press, which the cursor goes to before the page hears it. `acting` is that wait: a second Next in it is no second press.
  const [pressed, setPressed] = useState<WalkPress | undefined>();
  const acting = useRef(false);
  const steps = script.steps;

  const at = phase.is === "done" ? steps.length : phase.at;
  useEffect(() => {
    if (phase.is !== "seeking") return;
    let gone = false;
    void arrive(steps[at]!).then((element) => {
      if (gone) return;
      if (element === null) setPhase({ is: "stopped", at });
      // A step before the one asked to start on runs straight through.
      else if (at < from) {
        act(steps[at]!, element, steps[at + 1]);
        setPhase({ is: "seeking", at: at + 1 });
      } else setPhase({ is: "ready", at, element });
    });
    return () => {
      gone = true;
    };
  }, [phase.is, at, steps, from]);

  const next = useCallback(() => {
    if (phase.is !== "ready") return;
    // A bar that folds its tabs to icons as it measures replaces them after the step found its target, and a press on the
    // old one reaches nothing. Looked for again, as `arrive` does for one that went while it was waited on.
    if (acting.current) return;
    const element = phase.element.isConnected ? phase.element : (find(targetOf(steps[phase.at]!)) ?? phase.element);
    const step = steps[phase.at]!;
    const go = () => {
      acting.current = false;
      act(step, element, steps[phase.at + 1]);
      setPhase(phase.at + 1 < steps.length ? { is: "seeking", at: phase.at + 1 } : { is: "done" });
    };
    if (!("press" in step) || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return go();
    const here = element.getBoundingClientRect();
    setPressed((was) => ({ x: Math.round(here.left + here.width / 2), y: Math.round(here.top + here.height / 2), n: (was?.n ?? 0) + 1 }));
    acting.current = true;
    window.setTimeout(go, GLIDE_MS);
  }, [phase, steps]);

  /** The page again on step `to`, counted from 1: the steps before it run straight through. */
  const goTo = useCallback(
    (to: number) => {
      const url = new URL(window.location.href);
      if (to <= 1) url.searchParams.delete("step");
      else url.searchParams.set("step", String(Math.min(to, steps.length)));
      if (autoplay) url.searchParams.set("autoplay", "");
      else url.searchParams.delete("autoplay");
      window.location.assign(url);
    },
    [autoplay, steps.length],
  );
  const back = useCallback(() => {
    if (at > 0) goTo(at);
  }, [at, goTo]);
  const replay = useCallback(() => goTo(1), [goTo]);

  const controls = useRef({ next, back, replay, setAutoplay });
  controls.current = { next, back, replay, setAutoplay };

  // **Heard by the bar as well as the card.** Not a key a walk's own step sent: those are synthetic,
  // and a step that presses `r` is not asking to replay.
  useEffect(() => {
    const keys = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
      if (event.key === ".") {
        setFolded((was) => !was);
        return;
      }
      if (!event.isTrusted) return;
      const key = event.key.toLowerCase();
      // Enter on one of the card's own buttons presses it.
      if (key === "enter" && event.target instanceof HTMLElement && event.target.closest(`[${WALK_UI}] button`) !== null) return;
      if (event.repeat && (key === "a" || key === "r")) return;
      const run = controls.current;
      const act =
        key === "arrowright" || key === "enter"
          ? run.next
          : key === "arrowleft"
            ? run.back
            : key === "a"
              ? () => run.setAutoplay((was) => !was)
              : key === "r"
                ? run.replay
                : null;
      if (act === null) return;
      // Not also the press of whatever has focus.
      event.preventDefault();
      act();
    };
    window.addEventListener("keydown", keys);
    return () => window.removeEventListener("keydown", keys);
  }, []);

  const said = phase.is === "stopped" ? stopped(phase.at, steps[phase.at]!) : steps[Math.min(at, steps.length - 1)]!.say;
  const state = useMemo<WalkState>(
    () => ({ name, at: Math.min(at + 1, steps.length), of: steps.length, said, autoplay, ready: phase.is === "ready", done: phase.is === "done", stopped: phase.is === "stopped" }),
    [name, at, steps.length, said, autoplay, phase.is],
  );
  const latest = useRef(state);
  latest.current = state;
  const listeners = useRef(new Set<() => void>());
  useEffect(() => {
    for (const hear of listeners.current) hear();
  }, [state]);
  useEffect(() => {
    const handle: WalkHandle = {
      state: () => latest.current,
      subscribe: (hear) => {
        listeners.current.add(hear);
        return () => void listeners.current.delete(hear);
      },
      next: () => controls.current.next(),
      back: () => controls.current.back(),
      autoplay: (on) => controls.current.setAutoplay(on),
      replay: () => controls.current.replay(),
      hostCard: setHosted,
    };
    window.__armadaWalk = handle;
    return () => {
      if (window.__armadaWalk === handle) delete window.__armadaWalk;
    };
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
    // reaches the window the palette listens on. A card that closes on a press outside (`useCardOpen`)
    // listens for `pointerdown`, which Next would close the same way, so that one stops here too.
    <div {...{ [WALK_UI]: "" }} className="armada-mock-walk" onPointerDown={(event) => event.stopPropagation()} onMouseDown={(event) => {
        event.stopPropagation();
        // Nor does it take focus: a field the step before opened is still the one the next step types into.
        event.preventDefault();
      }}>
      {ring !== undefined && <div className="armada-mock-walk__ring" style={ring} />}
      <WalkPressMark press={pressed} />
      {!hosted && (
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
            <div className="armada-mock-walk__keyed">
              <Button size="sm" onClick={replay}>
                Replay {name}
              </Button>
              <Kbd>R</Kbd>
            </div>
          ) : folded ? (
            <div className="armada-mock-walk__pill" onPointerDown={drag}>
              <span className="armada-mock-walk__where">
                Step {at + 1} of {steps.length}
              </span>
              <span className="armada-mock-walk__keyed">
                <Tooltip label="Unfold the card">
                  <Button variant="ghost" size="sm" iconOnly aria-label="Unfold the card" onClick={() => setFolded(false)}>
                    <ChevronUp size={14} aria-hidden="true" />
                  </Button>
                </Tooltip>
                <Kbd>.</Kbd>
              </span>
              <span className="armada-mock-walk__keyed">
                <Button variant="primary" size="sm" disabled={phase.is !== "ready"} onClick={next}>
                  Next
                </Button>
                <Kbd>→</Kbd>
              </span>
            </div>
          ) : (
            <Card>
              <div className="armada-mock-walk__head" onPointerDown={drag}>
                <p className="armada-mock-walk__where">
                  <span className="armada-mock-walk__name">{name}</span>
                  <span>
                    {" · "}Step {at + 1} of {steps.length}
                  </span>
                </p>
                <span className="armada-mock-walk__keyed">
                  <Tooltip label="Replay from step 1">
                    <Button variant="ghost" size="sm" iconOnly aria-label="Replay from step 1" onClick={replay}>
                      <RotateCcw size={14} aria-hidden="true" />
                    </Button>
                  </Tooltip>
                  <Kbd>R</Kbd>
                  <Tooltip label="Fold the card">
                    <Button variant="ghost" size="sm" iconOnly aria-label="Fold the card" onClick={() => setFolded(true)}>
                      <ChevronDown size={14} aria-hidden="true" />
                    </Button>
                  </Tooltip>
                  <Kbd>.</Kbd>
                </span>
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
                  <span className="armada-mock-walk__keyed">
                    <Tooltip label="Replay to the step before">
                      <Button variant="ghost" size="sm" disabled={at < 1} onClick={back}>
                        Back
                      </Button>
                    </Tooltip>
                    <Kbd>←</Kbd>
                  </span>
                  <span className="armada-mock-walk__keyed">
                    <Tooltip label="Go on by itself">
                      <Switch checked={autoplay} onChange={(event) => setAutoplay(event.target.checked)}>
                        Autoplay
                      </Switch>
                    </Tooltip>
                    <Kbd>A</Kbd>
                  </span>
                  <span className="armada-mock-walk__keyed">
                    <Tooltip label="Act on the ringed target and go on">
                      <Button variant="primary" size="sm" disabled={phase.is !== "ready"} onClick={next}>
                        Next
                      </Button>
                    </Tooltip>
                    <Kbd>→</Kbd>
                  </span>
                </div>
              )}
            </Card>
          )}
        </div>
      )}
    </div>
  );
}

/** The walk's card and ring, in their own root over the app's. */
export function mountWalk(name: string, script: WalkScript, autoplay: boolean, host: HTMLElement, from = 0): void {
  createRoot(host).render(
    <StrictMode>
      <WalkPlayer name={name} script={script} autoplay={autoplay} from={from} />
    </StrictMode>,
  );
}

/**
 * A walk the page has no file for. **Its handle is a walk stopped on its one step**, so the window tells the Job rather than
 * reading the page as fine; its acts do nothing, and the card leaves to the bar as the player's does.
 */
function NoWalk({ name }: { name: string }) {
  const [hosted, setHosted] = useState(false);
  useEffect(() => {
    const state: WalkState = { name, at: 1, of: 1, said: `No walk named ${name}.`, autoplay: false, ready: false, done: false, stopped: true };
    const handle: WalkHandle = {
      state: () => state,
      subscribe: () => () => {},
      next: () => {},
      back: () => {},
      autoplay: () => {},
      replay: () => {},
      hostCard: setHosted,
    };
    window.__armadaWalk = handle;
    return () => {
      if (window.__armadaWalk === handle) delete window.__armadaWalk;
    };
  }, [name]);
  return (
    <div {...{ [WALK_UI]: "" }} className="armada-mock-walk">
      {hosted ? null : (
        <div className="armada-mock-walk__card" data-edge="bottom" data-walk-state="stopped">
          <Card>
            <div className="armada-mock-walk__said" data-stopped>
              <Alert>No walk named {name}.</Alert>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}

/** What the page draws for a walk it has no file for. */
export function mountNoWalk(name: string, host: HTMLElement): void {
  createRoot(host).render(<NoWalk name={name} />);
}

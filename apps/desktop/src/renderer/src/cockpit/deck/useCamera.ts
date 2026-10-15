// The map's camera as a hook: the state it is in (at rest on the whole field, held by a hand, pulled back to show an arrival), and the
// pointer, wheel, pinch and keys that move it. The maths is `camera.ts`; this is the part that touches the page. One state, one
// transform, drawn at most once a frame. A hand on it is `live` and follows at once; anything else eases, which the stylesheet turns
// off for reduced motion. Ported from proto/cockpit-flight-deck. #2301.

import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type RefObject } from "react";

import { DRAG, KEY_PAN, ZOOM_STEP, dragged, held as limited, panned, zoomed, type Cam, type Limits } from "./camera";
import type { Point, Size } from "./layout";
import { camera } from "./store";

export type CameraIn = {
  host: RefObject<HTMLDivElement | null>;
  size: Size;
  plane: Size;
  limits: Limits;
  /** The whole field, where it rests. */
  home: Cam;
  /** Where an arrival is shown whole from, and whether one is playing. */
  pull: Cam | undefined;
  arriving: boolean;
  /** The camera is the window's: where it was left is kept for the map to come back to. A mission's is its own. */
  keep: boolean;
  /** What the Home key and the locate button do. */
  locate: () => void;
  /** A hand is on the map: a callout is put away. */
  onHand: () => void;
};

export type Move = "hand" | "ease";

export function useCamera(input: CameraIn) {
  const { host, size, plane, limits, home, pull, arriving, keep, locate, onHand } = input;
  const fit = (c: Cam): Cam => limited(plane, limits, c);
  const [hand, setHand] = useState<Cam | undefined>(keep ? camera.current : undefined);
  const [live, setLive] = useState(false);
  const [grab, setGrab] = useState(false);
  // A dispatch or a merge that happens while he watches is shown whole: the camera pulls back to it, the event plays, it returns.
  const [pulled, setPulled] = useState(false);
  useEffect(() => {
    if (!arriving) return;
    setPulled(true);
    const timer = window.setTimeout(() => setPulled(false), 3600);
    return () => window.clearTimeout(timer);
  }, [arriving]);
  const resting = hand === undefined ? home : fit(hand);
  const eff = pulled && pull !== undefined ? pull : resting;
  const latest = useRef(eff);
  latest.current = eff;
  const frame = useRef<number>(0);
  const next = useRef<Cam>(eff);
  const quiet = useRef<number>(0);

  /** Move the camera. A hand on it is `live`: no easing, it follows at once. Anything else eases. */
  const go = (to: Cam, by: Move) => {
    const target = fit(to);
    latest.current = target;
    next.current = target;
    if (keep) camera.current = target;
    if (by === "hand") {
      setLive(true);
      window.clearTimeout(quiet.current);
      quiet.current = window.setTimeout(() => setLive(false), 180);
    } else {
      window.clearTimeout(quiet.current);
      setLive(false);
    }
    setPulled(false);
    if (frame.current === 0) {
      frame.current = window.requestAnimationFrame(() => {
        frame.current = 0;
        setHand(next.current);
      });
    }
  };
  useEffect(
    () => () => {
      window.cancelAnimationFrame(frame.current);
      window.clearTimeout(quiet.current);
    },
    [],
  );
  const centred = (): Point => {
    const box = host.current?.getBoundingClientRect();
    return box === undefined ? { x: 0, y: 0 } : { x: box.left + box.width / 2, y: box.top + box.height / 2 };
  };
  /** Zoom by a factor, keeping the point of the plane under the screen point (x, y) where it is. */
  const zoomAt = (factor: number, x: number, y: number, by: Move = "hand") => {
    const mid = centred();
    const to = zoomed(size, plane, limits, latest.current, factor, { x: x - mid.x, y: y - mid.y });
    if (to !== undefined) go(to, by);
  };
  const zoomMid = (factor: number) => {
    const mid = centred();
    zoomAt(factor, mid.x, mid.y, "ease");
  };
  /** Move the camera so what lay under the middle of the pane now lies a (dx, dy) of the screen away: a scroll, or an arrow key. */
  const panBy = (dx: number, dy: number, by: Move = "hand") => go(panned(size, plane, limits, latest.current, dx, dy), by);
  /** Back to the fit: not a camera of his own, so the field refits as the pane changes, and what he left is forgotten. */
  const homeAgain = () => {
    window.clearTimeout(quiet.current);
    window.cancelAnimationFrame(frame.current);
    frame.current = 0;
    if (keep) camera.current = undefined;
    latest.current = home;
    next.current = home;
    setLive(false);
    setPulled(false);
    setHand(undefined);
  };

  // The wheel and the pinch. A pinch arrives as a wheel with ctrl held; a mouse wheel as large whole steps; two fingers sliding as the small rest.
  const press = useRef<{ id: number; x: number; y: number; at: Cam; moved: boolean } | null>(null);
  const swallow = useRef(false);
  useEffect(() => {
    const one = host.current;
    if (one === null) return;
    const wheel = (event: WheelEvent) => {
      if ((event.target as Element).closest?.(".bf-zoom") !== null) return;
      event.preventDefault();
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 400 : 1;
      const dx = event.deltaX * unit;
      const dy = event.deltaY * unit;
      const wheelStep = event.deltaMode !== 0 || (dx === 0 && Math.abs(dy) >= 50 && Number.isInteger(dy));
      if (event.ctrlKey) zoomAt(Math.exp(-dy * 0.012), event.clientX, event.clientY);
      else if (wheelStep) zoomAt(Math.exp(-Math.sign(dy) * Math.min(Math.abs(dy), 160) * 0.0032), event.clientX, event.clientY);
      else panBy(dx, dy);
      onHand();
    };
    let from = 1;
    const began = (event: Event) => {
      event.preventDefault();
      from = latest.current.eye;
    };
    const changed = (event: Event) => {
      event.preventDefault();
      const at = centred();
      zoomAt((from * (event as unknown as { scale: number }).scale) / latest.current.eye, at.x, at.y);
    };
    one.addEventListener("wheel", wheel, { passive: false });
    one.addEventListener("gesturestart", began);
    one.addEventListener("gesturechange", changed);
    return () => {
      one.removeEventListener("wheel", wheel);
      one.removeEventListener("gesturestart", began);
      one.removeEventListener("gesturechange", changed);
    };
  });

  // The pointer: a press that moves less than DRAG is still a click on whatever it landed on; past it, it is a drag, and nothing is picked.
  const onDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if ((event.pointerType === "mouse" && event.button !== 0) || (event.target as Element).closest(".bf-zoom, .bf-edge") !== null) return;
    press.current = { id: event.pointerId, x: event.clientX, y: event.clientY, at: latest.current, moved: false };
  };
  const onMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const one = press.current;
    if (one === null || one.id !== event.pointerId) return;
    if (!one.moved) {
      if (Math.hypot(event.clientX - one.x, event.clientY - one.y) < DRAG) return;
      one.moved = true;
      host.current?.setPointerCapture(event.pointerId);
      setGrab(true);
      onHand();
    }
    const mid = centred();
    go(dragged(size, plane, limits, one.at, { x: one.x - mid.x, y: one.y - mid.y }, { x: event.clientX - mid.x, y: event.clientY - mid.y }), "hand");
  };
  const onUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const one = press.current;
    if (one === null || one.id !== event.pointerId) return;
    press.current = null;
    if (!one.moved) return;
    swallow.current = true;
    window.setTimeout(() => (swallow.current = false), 0);
    host.current?.releasePointerCapture?.(event.pointerId);
    setGrab(false);
  };
  const onClick = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (!swallow.current) return;
    event.stopPropagation();
    event.preventDefault();
  };
  const onDouble = (event: ReactMouseEvent<HTMLDivElement>) => {
    if ((event.target as Element).closest("[role=option], .bf-zoom, .bf-edge") === null) homeAgain();
  };
  // The keys: arrows pan, + and - zoom, 0 goes back, Home locates, and nothing here reaches the deck's own.
  const onKeys = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const step = KEY_PAN * (event.shiftKey ? 3 : 1);
    const pan: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    const move = pan[event.key];
    if (move !== undefined) panBy(move[0], move[1], "ease");
    else if (event.key === "+" || event.key === "=") zoomMid(ZOOM_STEP);
    else if (event.key === "-" || event.key === "_") zoomMid(1 / ZOOM_STEP);
    else if (event.key === "0") homeAgain();
    else if (event.key === "Home") locate();
    else return;
    event.preventDefault();
    event.stopPropagation();
  };

  return { eff, latest, live, grab, go, zoomMid, homeAgain, handlers: { onPointerDown: onDown, onPointerMove: onMove, onPointerUp: onUp, onPointerCancel: onUp, onClickCapture: onClick, onDoubleClick: onDouble, onKeyDown: onKeys } };
}

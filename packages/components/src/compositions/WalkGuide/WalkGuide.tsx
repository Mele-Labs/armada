import { useEffect, useRef, type CSSProperties } from "react";

import type { CaptureBox } from "../StudioCapture/StudioCapture";

/**
 * The ring and the caption of a walk Bridge plays itself, `show_window`'s `steps` —
 * `docs/practices/capture-window.md`, *A walk in the page*. The page being walked draws nothing of
 * it, since it knows nothing of Armada: this is drawn by Bridge's bar over the page, as a Session's
 * question about a place is, in the look of the mock's walk.
 *
 * **It draws and it does not look.** Where the target is comes from main, and what the buttons do is
 * the bar's row above: Back, Next, Autoplay, Replay. A press on the page beside the card is a person
 * reaching for it, so the ring goes away and lets them.
 */
/** Where the last press landed, in this document's coordinates. `n` counts the presses, so each one plays the cursor again. */
export type WalkPress = { x: number; y: number; n: number };

export type WalkGuideProps = {
  /** The walk's name, as the window is titled. */
  name: string;
  /** The step on screen, counted from 1, of this many. */
  at: number;
  of: number;
  /** The step's caption. */
  said: string;
  /** The target, in this document's own coordinates. `null`: none is ringed yet. */
  box: CaptureBox | null;
  /** How far down the page starts, so a card at the top rests under the bar. */
  top: number;
  /** The last press of the walk: a cursor goes to it and rings grow from it. Absent: no press has been made. */
  press?: WalkPress;
  /** Put the ring away so the page takes presses. */
  onFold: () => void;
};

/** How far off the first cursor starts, since there is no earlier press to come from. */
const ENTRY = 56;

const reducedMotion = (): boolean => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The cursor glides from where the last press landed, then rings grow out of the point and fade. CSS only: the keys restart it. */
function Press({ press, from }: { press: WalkPress; from: { x: number; y: number } | null }) {
  const start = from ?? { x: press.x - ENTRY, y: press.y - ENTRY };
  return (
    <div
      key={press.n}
      className="armada-walk-guide__press"
      aria-hidden
      style={{ "--to-x": `${press.x}px`, "--to-y": `${press.y}px`, "--from-x": `${start.x}px`, "--from-y": `${start.y}px` } as CSSProperties}
    >
      <svg className="armada-walk-guide__cursor" width="16" height="20" viewBox="0 0 16 20">
        <path d="M1 1 L1 15 L5 11.5 L8 18 L10.5 17 L7.5 10.5 L13 10.5 Z" />
      </svg>
      <span className="armada-walk-guide__pulse" />
      <span className="armada-walk-guide__pulse" />
    </div>
  );
}

const place = (box: CaptureBox): CSSProperties =>
  ({
    "--walk-x": `${box.x}px`,
    "--walk-y": `${box.y}px`,
    "--walk-w": `${box.width}px`,
    "--walk-h": `${box.height}px`,
  }) as CSSProperties;

/**
 * The trailing bottom corner, and the trailing top one when the target is down there: a card never
 * covers what it points at. The mock's `placeCard`, by where the target's middle is.
 */
function edgeFor(box: CaptureBox | null, view: { width: number; height: number }): "bottom" | "top" {
  if (box === null) return "bottom";
  const middle = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  return middle.x > view.width / 2 && middle.y > view.height * 0.55 ? "top" : "bottom";
}

export function WalkGuide({ name, at, of, said, box, top, press, onFold }: WalkGuideProps) {
  // The point the next press comes from, kept after the render that drew this one.
  const last = useRef<{ x: number; y: number } | null>(null);
  const from = last.current;
  useEffect(() => {
    if (press !== undefined) last.current = { x: press.x, y: press.y };
  }, [press?.n]);
  return (
    <div className="armada-walk-guide" data-armada-capture="" style={{ "--walk-top": `${top}px` } as CSSProperties}>
      <div className="armada-walk-guide__catch" aria-hidden onClick={onFold} />
      {box === null ? null : <div className="armada-walk-guide__ring" role="img" aria-label="The target of this step" style={place(box)} />}
      {press === undefined || reducedMotion() ? null : <Press press={press} from={from} />}
      <div className="armada-walk-guide__card" role="status" data-edge={edgeFor(box, { width: window.innerWidth, height: window.innerHeight })}>
        <p className="armada-walk-guide__where">
          <span className="armada-walk-guide__name">{name}</span>
          <span>
            {" · "}Step {at} of {of}
          </span>
        </p>
        <p className="armada-walk-guide__said">{said}</p>
      </div>
    </div>
  );
}

import type { CSSProperties } from "react";

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
  /** Put the ring away so the page takes presses. */
  onFold: () => void;
};

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

export function WalkGuide({ name, at, of, said, box, top, onFold }: WalkGuideProps) {
  return (
    <div className="armada-walk-guide" data-armada-capture="" style={{ "--walk-top": `${top}px` } as CSSProperties}>
      <div className="armada-walk-guide__catch" aria-hidden onClick={onFold} />
      {box === null ? null : <div className="armada-walk-guide__ring" role="img" aria-label="The target of this step" style={place(box)} />}
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

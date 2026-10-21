import { useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { KbdChord } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * Studio capture — the layer a person points through to put a Note on the open
 * Studio. `docs/concepts/studio.md`, Notes; `#1290`.
 *
 * **It draws and it does not look.** What is under the pointer, what was
 * pressed and what the Note keeps are read from the page by
 * `apps/desktop`'s capture; this takes boxes and text and puts them on screen.
 *
 * **The card sits against the element, and reading and writing are one place.**
 * A person points, sees which component they hit, and types into the card over
 * it — never a panel elsewhere.
 */

/** A rectangle in the window, in CSS pixels. */
export type CaptureBox = { x: number; y: number; width: number; height: number };

export type StudioCaptureProps = {
  /** What the pointer is over, and what to call it. Absent once one is held. */
  hovered?: { box: CaptureBox; named: string };
  /** The element the note is being written on, and the chain that found it. */
  held?: { box: CaptureBox; chain: string };
  /** What is being typed. Controlled: the layer owns the draft. */
  note: string;
  onNote: (note: string) => void;
  onSave: () => void;
  onCancel: () => void;
  /** Out to Fleet: the card waits rather than taking a second press. */
  saving?: boolean;
  /** Why the last capture did not land, or absent. */
  refused?: string;
  /** What capture is aimed at — the Studio's name — or why it is aimed at nothing. */
  aim: string;
  /** Whether `aim` names a Studio. A capture with none saves nothing. */
  aimed: boolean;
  /** The binding that turns capture off, from the action registry. */
  binding: readonly string[];
};

const place = (box: CaptureBox): CSSProperties =>
  ({
    "--capture-x": `${box.x}px`,
    "--capture-y": `${box.y}px`,
    "--capture-w": `${box.width}px`,
    "--capture-h": `${box.height}px`,
  }) as CSSProperties;

type Spot = { left: number; top: number; side: "below" | "above" };

const within = (low: number, wanted: number, high: number): number => Math.max(low, Math.min(wanted, Math.max(low, high)));

/**
 * Under the element where the box it is drawn in has room below it, over it
 * where only above has room, and pulled in on every side so no edge is cut.
 * Everything is in the box's own pixels: the element's box was measured there,
 * and the window the box sits in is not the room the card has.
 */
function cardSpot(box: CaptureBox, card: { width: number; height: number }, room: { width: number; height: number }, gap: number): Spot {
  const below = box.y + box.height + gap;
  const above = box.y - gap - card.height;
  const fitsBelow = below + card.height + gap <= room.height;
  const fitsAbove = above >= gap;
  // With room on neither side the card goes where there is more, and the clamp below keeps it in.
  const roomier = room.height - (box.y + box.height) >= box.y ? "below" : "above";
  const side = fitsBelow ? "below" : fitsAbove ? "above" : roomier;
  const wanted = side === "below" ? below : above;
  return {
    left: within(gap, box.x, room.width - card.width - gap),
    top: within(gap, wanted, room.height - card.height - gap),
    side,
  };
}

export function StudioCapture(props: StudioCaptureProps) {
  const { hovered, held, note, onNote, onSave, onCancel, saving = false, refused, aim, aimed, binding } = props;
  const savable = aimed && !saving && note.trim() !== "";

  function keyed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onCancel();
    } else if (event.key === "Enter" && event.metaKey && savable) {
      event.preventDefault();
      event.stopPropagation();
      onSave();
    }
  }

  const root = useRef<HTMLDivElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const [spot, setSpot] = useState<Spot | null>(null);
  const box = held?.box;

  // Placed from the box the layer is drawn in and the card as it is now, so a
  // window's page column, not the screen, is the room it has.
  useLayoutEffect(() => {
    const layer = root.current;
    const card = cardRef.current;
    if (layer === null || card === null || box === undefined) return;
    const settle = (): void => {
      const gap = Number.parseFloat(getComputedStyle(card).marginTop) || 0;
      const next = cardSpot(
        box,
        { width: card.offsetWidth, height: card.offsetHeight },
        { width: layer.clientWidth, height: layer.clientHeight },
        gap,
      );
      // The card's margin is part of its offset, so the spot is where its margin edge goes.
      const at = { left: next.left - gap, top: next.top - gap, side: next.side };
      setSpot((was) => (was?.left === at.left && was.top === at.top && was.side === at.side ? was : at));
    };
    settle();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(settle);
    watch.observe(layer);
    watch.observe(card);
    return () => watch.disconnect();
  }, [held !== undefined, box?.x, box?.y, box?.width, box?.height]);

  // The card opens on a press, so the cursor belongs in the note without a
  // second one. Not `autoFocus`: the card is hidden until placed, and a hidden
  // field refuses focus.
  const placed = held !== undefined && spot !== null;
  useLayoutEffect(() => {
    if (placed) cardRef.current?.querySelector("textarea")?.focus();
  }, [placed]);

  return (
    <div ref={root} className="armada-capture" data-armada-capture="" onKeyDown={keyed}>
      {hovered === undefined || held !== undefined ? null : (
        <div className="armada-capture__outline" style={place(hovered.box)}>
          <span className="armada-capture__named">{hovered.named}</span>
        </div>
      )}

      {held === undefined ? null : (
        <>
          <div className="armada-capture__outline" data-held="" style={place(held.box)} />
          <div
            ref={cardRef}
            className="armada-capture__card"
            role="dialog"
            aria-label="Capture a note"
            style={spot === null ? { visibility: "hidden" } : { left: spot.left, top: spot.top }}
            data-side={spot?.side}
          >
            <p className="armada-capture__chain">{held.chain}</p>
            <Textarea label="Note" value={note} onChange={(event) => onNote(event.target.value)} />
            {refused === undefined ? null : <p className="armada-capture__refused">{refused}</p>}
            <div className="armada-capture__acts">
              <Button variant="ghost" size="sm" onClick={onCancel}>
                Cancel
              </Button>
              <Button variant="primary" size="sm" pending={saving} disabled={!savable} onClick={onSave}>
                {saving ? "Capturing" : "Capture"}
              </Button>
            </div>
          </div>
        </>
      )}

      <div className="armada-capture__bar" role="status">
        <span>Capturing</span>
        <span>{aim}</span>
        <KbdChord keys={[...binding]} aria-label={`${binding.join(" ")} turns capturing off`} />
      </div>
    </div>
  );
}

// The notes held on the Job beside the page, drawn as the last entry of its thread: a turn of the owner's that is not sent
// and grows as notes are taken, each note with its crop and its words, the words editable, each removable, and one Send at
// its foot that sends them all as one turn. `docs/practices/capture-window.md`, *Pending notes*. The walk window's panel
// draws it from main's list, and the mock's walk window from its own.

import { useEffect, useState } from "react";
import type { CSSProperties } from "react";
import { CircleDashed } from "lucide-react";
import { Button, Textarea, Tooltip } from "@armada/components";
import type { PanelHeldNote } from "@armada/studios/api";

import "./held-notes.css";

/** What an act came back with: done, or the words for why not. */
export type HeldAnswer = { ok: true } | { ok: false; said: string };

export type HeldNotesActs = {
  onEdit: (id: string, said: string) => Promise<HeldAnswer>;
  onRemove: (id: string) => Promise<HeldAnswer>;
  onSend: () => Promise<HeldAnswer>;
};

export function HeldNotes({ notes, onEdit, onRemove, onSend }: { notes: readonly PanelHeldNote[] } & HeldNotesActs) {
  const [sending, setSending] = useState(false);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  if (notes.length === 0) return null;
  async function send(): Promise<void> {
    setSending(true);
    const answer = await onSend();
    setSending(false);
    setRefused(answer.ok ? undefined : `Not sent: ${answer.said}`);
  }
  return (
    <section className="armada-held-notes" aria-label="Pending notes">
      <header className="armada-held-notes__band">
        <span className="armada-held-notes__who">You</span>
        <Tooltip label="Pending">
          <span className="armada-held-notes__pending" role="img" aria-label="Pending">
            <CircleDashed size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      </header>
      <div className="armada-held-notes__body">
        <ol className="armada-held-notes__list">
          {notes.map((one, at) => (
            <HeldNoteRow key={one.id} note={one} number={at + 1} onEdit={onEdit} onRemove={onRemove} />
          ))}
        </ol>
        {refused === undefined ? null : (
          <p className="armada-held-notes__refused" role="alert">
            {refused}
          </p>
        )}
        <div className="armada-held-notes__foot">
          <Button variant="primary" size="sm" pending={sending} disabled={sending} onClick={() => void send()}>
            Send notes
          </Button>
        </div>
      </div>
    </section>
  );
}

function HeldNoteRow({ note, number, onEdit, onRemove }: { note: PanelHeldNote; number: number } & Pick<HeldNotesActs, "onEdit" | "onRemove">) {
  const [text, setText] = useState(note.said);
  // Main's words win once they change there: an edit taken, or the same note told again.
  useEffect(() => setText(note.said), [note.said]);
  // Kept as it is left: words that differ go to main, and nothing typed puts the note's own words back.
  function leave(): void {
    const words = text.trim();
    if (words === "") setText(note.said);
    else if (words !== note.said) void onEdit(note.id, words);
  }
  return (
    <li className="armada-held-notes__note" aria-label={note.said}>
      {note.crop === undefined ? null : <Crop crop={note.crop} ring={note.ring} number={number} on={note.on} />}
      <div className="armada-held-notes__fields">
        <span className="armada-held-notes__on">On {note.on}</span>
        <Textarea label="Note" keep={`held-note:${note.id}`} rows={2} grow value={text} onChange={(event) => setText(event.target.value)} onBlur={leave} />
      </div>
      <Button variant="ghost" size="sm" aria-label={`Remove the note “${note.said}”`} onClick={() => void onRemove(note.id)}>
        Remove
      </Button>
    </li>
  );
}

/**
 * The crop main took, drawn from a blob: the panel's CSP takes `blob:` images and no `data:` ones. With a ring it is the
 * whole window, shrunk to the panel's width, the box ringed over it and tagged with the note's number in the turn sent.
 */
function Crop({ crop, ring, number, on }: { crop: NonNullable<PanelHeldNote["crop"]>; ring: PanelHeldNote["ring"]; number: number; on: string }) {
  const [src, setSrc] = useState<string | null>(null);
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  useEffect(() => {
    const bytes = Uint8Array.from(atob(crop.data), (one) => one.charCodeAt(0));
    const address = URL.createObjectURL(new Blob([bytes], { type: crop.media_type }));
    setSrc(address);
    return () => URL.revokeObjectURL(address);
  }, [crop.data, crop.media_type]);
  if (src === null) return null;
  if (ring === undefined) return <img className="armada-held-notes__crop" src={src} alt={`Crop of ${on}`} />;
  // The box as fractions of the picture, so it stays on the element at whatever width the panel draws it.
  const at =
    size === null
      ? null
      : ({
          "--ring-x": `${(ring.x / size.width) * 100}%`,
          "--ring-y": `${(ring.y / size.height) * 100}%`,
          "--ring-width": `${(ring.width / size.width) * 100}%`,
          "--ring-height": `${(ring.height / size.height) * 100}%`,
        } as CSSProperties);
  return (
    <div className="armada-held-notes__shot">
      <img
        className="armada-held-notes__window"
        src={src}
        alt={`Window, ${on} ringed`}
        onLoad={(event) => setSize({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight })}
      />
      {at === null ? null : (
        <span className="armada-held-notes__ring" style={at}>
          <span className="armada-held-notes__tag">
            <Tooltip label={`Note ${number}`}>
              <span role="img" aria-label={`Note ${number}`}>
                {number}
              </span>
            </Tooltip>
          </span>
        </span>
      )}
    </div>
  );
}

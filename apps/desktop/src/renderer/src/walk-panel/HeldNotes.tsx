// The notes held on the Job beside the page, above its view: each with its crop and its words, the words editable, each
// removable, and one Send that sends them all as one turn. `docs/practices/capture-window.md`, *Held notes*. The walk window's
// panel draws it over main's list, and the mock's walk window over its own.

import { useEffect, useState } from "react";
import { Button, Textarea } from "@armada/components";
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
    <section className="armada-held-notes" aria-label="Held notes">
      <header className="armada-held-notes__head">
        <h2 className="armada-held-notes__title">Held notes</h2>
        <Button variant="primary" size="sm" pending={sending} disabled={sending} onClick={() => void send()}>
          Send notes
        </Button>
      </header>
      {refused === undefined ? null : (
        <p className="armada-held-notes__refused" role="alert">
          {refused}
        </p>
      )}
      <ol className="armada-held-notes__list">
        {notes.map((one) => (
          <HeldNoteRow key={one.id} note={one} onEdit={onEdit} onRemove={onRemove} />
        ))}
      </ol>
    </section>
  );
}

function HeldNoteRow({ note, onEdit, onRemove }: { note: PanelHeldNote } & Pick<HeldNotesActs, "onEdit" | "onRemove">) {
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
    <li className="armada-held-notes__note" aria-label={`Note on ${note.on}`}>
      {note.crop === undefined ? null : <Crop crop={note.crop} on={note.on} />}
      <div className="armada-held-notes__body">
        <span className="armada-held-notes__on">On {note.on}</span>
        <Textarea label="Note" keep={`held-note:${note.id}`} rows={2} grow value={text} onChange={(event) => setText(event.target.value)} onBlur={leave} />
      </div>
      <Button variant="ghost" size="sm" aria-label={`Remove the note on ${note.on}`} onClick={() => void onRemove(note.id)}>
        Remove
      </Button>
    </li>
  );
}

/** The crop main took of the element, drawn from a blob: the panel's CSP takes `blob:` images and no `data:` ones. */
function Crop({ crop, on }: { crop: NonNullable<PanelHeldNote["crop"]>; on: string }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    const bytes = Uint8Array.from(atob(crop.data), (one) => one.charCodeAt(0));
    const address = URL.createObjectURL(new Blob([bytes], { type: crop.media_type }));
    setSrc(address);
    return () => URL.revokeObjectURL(address);
  }, [crop.data, crop.media_type]);
  return src === null ? null : <img className="armada-held-notes__crop" src={src} alt={`Crop of ${on}`} />;
}

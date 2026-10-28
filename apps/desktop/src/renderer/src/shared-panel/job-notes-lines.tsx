// The owner's notes for the Job, drawn on the Files tab's diff: a gutter press opens a box under the line, and a note kept
// hangs under the line it was written on. **They are for the Job alone**: a note is never added to a review or posted.

import { useMemo, useState } from "react";
import { Check, Notebook, Pencil, X } from "lucide-react";
import { Button, Prose, PrMark, SplitButton, Tooltip } from "@armada/components";
import type { DiffFile, DiffLine, UnifiedDiffProps } from "@armada/components";

import { LineComposer } from "../pr-review/lines";
import { counted, placeOfLine } from "../pr-review/patch";
import type { Counted } from "../pr-review/patch";
import "../pr-review/pr-review.css";
import { addNote, editNote, removeNote, useJobNotes } from "./job-notes";
import type { JobNote, NoteHandle } from "./job-notes";

type Composing = { path: string; index: number };

/** A file's lines numbered once, however many of its lines are asked about. */
const numbered = new WeakMap<readonly DiffLine[], Counted[]>();
function placeAt(lines: readonly DiffLine[], index: number) {
  if (!numbered.has(lines)) numbered.set(lines, counted(lines));
  const at = numbered.get(lines)![index];
  if (at === undefined) return undefined;
  if (at.now !== undefined) return { line: at.now, side: "right" as const };
  return at.old === undefined ? undefined : { line: at.old, side: "left" as const };
}

/** The note a line carries, as the forge would name its place. */
const hereOf = (note: JobNote, file: DiffFile, index: number): boolean => {
  const place = placeAt(file.lines, index);
  return place !== undefined && note.path === file.path && note.line === place.line && note.side === place.side;
};

export type LineNotes = {
  /** What the diff takes to draw the gutter and the notes. Absent where Fleet serves no notes. */
  lines: Pick<UnifiedDiffProps, "onLine" | "under"> | undefined;
  /** Every note for the pull request, resolved ones included. */
  notes: readonly JobNote[];
  showResolved: boolean;
  toggleResolved: () => void;
};

/**
 * What the Files tab's diff takes to draw his notes: the gutter and what sits under a line. **The notes are redrawn as they
 * change.** **A note the Job resolved is folded away** until Show resolved is pressed.
 */
export function useLineNotes(handle: NoteHandle | undefined): LineNotes {
  const notes = useJobNotes(handle);
  const [composing, setComposing] = useState<Composing | undefined>(undefined);
  const [showResolved, setShowResolved] = useState(false);
  const lines = useMemo(
    () =>
      handle === undefined
        ? undefined
        : {
            onLine: {
              label: "Note for the job",
              name: (file: DiffFile, index: number) => `Note for the job on ${file.lines[index]?.text.trim() ?? "this line"}`,
              press: (file: DiffFile, index: number) => setComposing({ path: file.path, index }),
            },
            under: (file: DiffFile, index: number) => {
              const here = notes.filter((one) => (showResolved || !one.done) && hereOf(one, file, index));
              const writing = composing?.path === file.path && composing.index === index;
              if (here.length === 0 && !writing) return null;
              const place = placeOfLine(file.lines, index);
              return (
                <div className="pr-under">
                  {here.map((one) => (
                    <NoteCard key={one.id} note={one} onRemove={() => void removeNote(handle, one.id)} onEdit={(body) => void editNote(handle, one.id, body)} />
                  ))}
                  {writing && place !== undefined ? (
                    <LineComposer
                      keep={`job-notes:${handle.manifestId}#${handle.number}:${file.path}:${index}`}
                      label="Note for the job"
                      action="Save note"
                      onAdd={(body) => {
                        void addNote(handle, { path: file.path, line: place.line, side: place.side, body });
                        setComposing(undefined);
                      }}
                      onCancel={() => setComposing(undefined)}
                    />
                  ) : null}
                </div>
              );
            },
          },
    // The handle is rebuilt as the panel draws; the pull request and the way Fleet is asked are what the notes follow.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [handle?.manifestId, handle?.number, handle?.reads, notes, composing, showResolved],
  );
  return { lines, notes, showResolved, toggleResolved: () => setShowResolved((was) => !was) };
}

/** A running Job the notes can go to instead of the one that holds the pull request. */
export type NoteTarget = { id: string; title: string };

/**
 * What waits to be sent, and where it goes. **The face sends to the Job that holds the pull request**, and the caret lists the other
 * running Jobs. Where none holds it the face is off and the caret alone is live.
 */
export function NotesBar({ count, holder, others, onSend }: { count: number; holder: string | undefined; others: readonly NoteTarget[]; onSend: (to?: string) => void }) {
  const notes = count === 1 ? "1 note" : `${count} notes`;
  return (
    <div className="pr-notes-bar" role="group" aria-label="Notes for the job">
      <span>{holder === undefined ? `${notes} kept, choose a job` : `${notes} for ${holder}`}</span>
      <SplitButton size="sm" menuLabel="Send to a job" faceDisabled={holder === undefined} onAction={() => onSend()} items={others.map((one) => ({ label: one.title, onSelect: () => onSend(one.id) }))}>
        Send to job
      </SplitButton>
    </div>
  );
}

/** Folds the notes a Job resolved away, or draws them again. Drawn only where one has. */
export function ResolvedToggle({ count, shown, onToggle }: { count: number; shown: boolean; onToggle: () => void }) {
  return (
    <Button variant="ghost" size="sm" aria-pressed={shown} onClick={onToggle}>
      {shown ? "Hide resolved" : `Show resolved (${count})`}
    </Button>
  );
}

function NoteCard({ note, onRemove, onEdit }: { note: JobNote; onRemove: () => void; onEdit: (body: string) => void }) {
  const [editing, setEditing] = useState(false);
  if (editing)
    return (
      <LineComposer
        keep={`job-notes:edit:${note.id}`}
        label="Note for the job"
        action="Save note"
        initial={note.body}
        onAdd={(body) => {
          onEdit(body);
          setEditing(false);
        }}
        onCancel={() => setEditing(false)}
      />
    );
  const to = note.title === undefined ? "the job" : note.title;
  return (
    <article className="pr-draft" aria-label="Note for the job" data-resolved={note.done || undefined}>
      <header className="pr-card__head">
        {note.done ? <PrMark Glyph={Check} said={`Resolved by ${to}`} /> : note.sent ? <PrMark Glyph={Check} said={`Sent to ${to}`} /> : <PrMark Glyph={Notebook} said="Kept on this Mac, not sent yet" />}
        <b className="pr-card__who">{note.done ? `Resolved by ${to}` : "Note for the job"}</b>
        <span className="pr-card__act">
          {note.sent ? null : (
            <Tooltip label="Edit the note">
              <Button variant="ghost" size="sm" iconOnly aria-label="Edit the note" onClick={() => setEditing(true)}>
                <Pencil size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
          <Tooltip label="Remove the note">
            <Button variant="ghost" size="sm" iconOnly aria-label="Remove the note" onClick={onRemove}>
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="pr-card__body">
        <Prose text={note.body} />
      </div>
    </article>
  );
}

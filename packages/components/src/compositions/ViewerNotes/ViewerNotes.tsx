import { X } from "lucide-react";
import { createContext, useContext, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";

import { forgetKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Kbd, KbdBinding } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { aboutSaid, notesMessage, type HeldNote, type NoteAbout, type NotesMessage } from "./compose";
import { setHeldOn, useHeldOn } from "./held";

/**
 * Notes on what is being read, sent to a Job without leaving it. A right press (or the menu key) on what a viewer has
 * pointed at offers Add note; the box that opens holds the note on the viewer with Enter, and ⌘Enter sends every held
 * note on it as one numbered message. **The viewers own what is pointed at and how a held note is marked**: a sketch, a
 * file and a picture each pass their `NoteAbout`; this owns the menu, the box, the held list and the send.
 */

/** Where notes go: the live Jobs a viewer outside one may pick, and the send. Absent, no viewer offers notes. */
export type ViewerNotesHost = {
  jobs: readonly { id: string; title: string }[];
  /** False where the message did not arrive; the notes stay held. */
  send: (job: string, message: NotesMessage) => Promise<boolean> | boolean | void;
};

const Host = createContext<ViewerNotesHost | undefined>(undefined);

export function ViewerNotesFrom({ host, children }: { host: ViewerNotesHost | undefined; children: ReactNode }) {
  return <Host.Provider value={host}>{children}</Host.Provider>;
}

/** What a viewer is showing: the key its notes are held under, and the Job it was opened in, where it was. */
export type ViewerNotesScope = { key: string; job?: string };

type Point = { x: number; y: number };
/** What was pointed at, on screen: the box opens clear of it. */
export type NoteAnchor = {
  left: number;
  top: number;
  right: number;
  bottom: number;
};

/** A note as a viewer marks it: its number, what it is about, and whether it is the one being written. */
export type NoteMark = {
  n: number;
  about: NoteAbout;
  text: string;
  writing: boolean;
};

export type ViewerNotes = {
  /** A host and a scope are both given: the viewer offers notes. */
  on: boolean;
  marks: readonly NoteMark[];
  /** A right press, or the menu key: the menu at `at`, about what is pointed at there, which stands at `anchor`. */
  menu: (at: Point, about: NoteAbout | undefined, anchor?: NoteAnchor) => void;
  /** Counts each hold and each send, so a viewer lets go of what it had drawn for the note. */
  settled: number;
  /** ⌘Enter on the viewer: every held note goes. */
  onKeyDown: (event: KeyboardEvent) => void;
  /** The menu and the box, drawn where the viewer puts them. */
  layer: ReactNode;
};

/** A press in the menu or the box is not a press on what it floats over: a scrim that closes on a press stays open. */
const keep = (event: { stopPropagation: () => void }) => event.stopPropagation();

/** The outermost fixed layer the viewer is drawn in: a sheet's or a picture's, so the box floats over it. */
function layerOf(element: HTMLElement | null): HTMLElement {
  let found: HTMLElement | null = null;
  for (let at = element?.parentElement ?? null; at !== null && at !== document.body; at = at.parentElement) {
    if (getComputedStyle(at).position === "fixed") found = at;
  }
  return found ?? document.body;
}

const NO_MARKS: readonly NoteMark[] = [];

/** Where the menu or the box was opened, about what, standing where. */
type Opened = { at: Point; about: NoteAbout | undefined; anchor: NoteAnchor | undefined };

const sends = (event: { key: string; metaKey: boolean; ctrlKey: boolean }) => event.key === "Enter" && (event.metaKey || event.ctrlKey);

/** The notes of one viewer. `from` is the viewer, where focus goes back to unless `back` says otherwise. */
export function useViewerNotes(scope: ViewerNotesScope | undefined, from: RefObject<HTMLElement | null>, back?: () => void): ViewerNotes {
  const host = useContext(Host);
  const on = host !== undefined && scope !== undefined;
  const { notes: held, job: picked } = useHeldOn(on ? scope.key : undefined);
  const job = scope?.job ?? picked;
  const [menuAt, setMenuAt] = useState<Opened | undefined>(undefined);
  const [box, setBox] = useState<Opened | undefined>(undefined);
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const [settled, setSettled] = useState(0);
  // The menu and the box are drawn in a layer of their own, which a sheet's Esc leaves to them.
  const [layerEl, setLayerEl] = useState<HTMLDivElement | null>(null);
  const last = useRef<Point>({ x: 0, y: 0 });

  const toViewer = () => (back === undefined ? from.current?.focus() : back());
  const close = () => {
    setBox(undefined);
    toViewer();
  };

  // The box closes in the same render the text empties, so the field never sees it go empty and would keep the old draft.
  const forgetDraft = (key: string) => {
    setText("");
    forgetKept(`note:${key}`);
  };

  const send = async () => {
    if (!on || sending) return;
    const writing = box?.about !== undefined && text.trim() !== "" ? [{ about: box.about, text: text.trim() }] : [];
    const all: HeldNote[] = [...held, ...writing];
    if (all.length === 0) return;
    // Nowhere to send yet: the box opens on the held notes, with the Job to pick.
    if (job === undefined) {
      if (box === undefined) setBox({ at: last.current, about: undefined, anchor: undefined });
      return;
    }
    setSending(true);
    const went = await host.send(job, notesMessage(all));
    setSending(false);
    if (went === false) return;
    setHeldOn(scope.key, (was) => ({ ...was, notes: [] }));
    forgetDraft(scope.key);
    setSettled((was) => was + 1);
    close();
  };

  const hold = () => {
    if (!on || box?.about === undefined || text.trim() === "") return;
    const about = box.about;
    setHeldOn(scope.key, (was) => ({ ...was, notes: [...was.notes, { about, text: text.trim() }] }));
    forgetDraft(scope.key);
    setSettled((was) => was + 1);
    close();
  };

  // Kept between renders while nothing in it changes, so a viewer that redraws on it redraws only then.
  const writingAbout = box?.about;
  const marks = useMemo<NoteMark[]>(
    () => [
      ...held.map((one, at) => ({ n: at + 1, about: one.about, text: one.text, writing: false })),
      ...(writingAbout === undefined ? [] : [{ n: held.length + 1, about: writingAbout, text, writing: true }]),
    ],
    [held, writingAbout, text],
  );

  const entries: DropdownMenuEntry[] = [
    ...(menuAt?.about === undefined ? [] : [{ kind: "item" as const, id: "add", label: "Add note" }]),
    ...(held.length === 0 ? [] : [{ kind: "item" as const, id: "send", label: "Send notes", shortcut: "⌘↵" }]),
  ];

  const where = layerOf(from.current);
  const layer = !on ? null : (
    <span
      className="armada-viewer-notes"
      data-viewer-notes-open={menuAt !== undefined || box !== undefined || undefined}
      onClick={keep}
      onPointerDown={keep}
      onContextMenu={keep}
    >
      {menuAt === undefined && box === undefined
        ? null
        : createPortal(
            <div ref={setLayerEl} className="armada-viewer-notes__layer" data-keeps-esc="">
              {menuAt === undefined || entries.length === 0 || layerEl === null ? null : (
                <DropdownMenu
                  triggerLabel="Note"
                  at={menuAt.at}
                  portal={layerEl}
                  open
                  entries={entries}
                  onOpenChange={(now) => {
                    if (now) return;
                    setMenuAt(undefined);
                  }}
                  onSelect={(id) => {
                    if (id === "add") setBox(menuAt);
                    else void send();
                  }}
                />
              )}
              {box === undefined ? null : (
                <NoteBox
                  at={box.at}
                  anchor={box.anchor}
                  about={box.about}
                  held={held}
                  text={text}
                  keepKey={`note:${scope.key}`}
                  jobs={scope.job === undefined ? host.jobs : undefined}
                  job={job}
                  sending={sending}
                  onText={setText}
                  onHold={hold}
                  onSend={() => void send()}
                  onPick={(id) => setHeldOn(scope.key, (was) => ({ ...was, job: id }))}
                  onRemove={(at) =>
                    setHeldOn(scope.key, (was) => ({
                      ...was,
                      notes: was.notes.filter((_, one) => one !== at),
                    }))
                  }
                  onClose={close}
                />
              )}
            </div>,
            where,
          )}
    </span>
  );

  return {
    on,
    marks: on ? marks : NO_MARKS,
    menu: (at, about, anchor) => {
      if (!on) return;
      last.current = at;
      if (about === undefined && held.length === 0) return;
      setBox(undefined);
      setMenuAt({ at, about, anchor });
    },
    settled,
    onKeyDown: (event) => {
      if (!on || event.defaultPrevented || !sends(event) || held.length === 0) return;
      event.preventDefault();
      void send();
    },
    layer,
  };
}

/** A note's number, as a viewer marks the place it is about and as the box lists it. */
export function NoteNumber({ n, writing = false, said }: { n: number; writing?: boolean; said?: string }) {
  const mark = (
    <span className="armada-note-number" data-writing={writing || undefined} role="img" aria-label={`Note ${String(n)}`}>
      {n}
    </span>
  );
  return said === undefined || said === "" ? mark : <Tooltip label={said}>{mark}</Tooltip>;
}

function NoteBox({
  at,
  anchor,
  about,
  held,
  text,
  keepKey,
  jobs,
  job,
  sending,
  onText,
  onHold,
  onSend,
  onPick,
  onRemove,
  onClose,
}: {
  at: Point;
  anchor: NoteAnchor | undefined;
  about: NoteAbout | undefined;
  held: readonly HeldNote[];
  text: string;
  keepKey: string;
  /** The Jobs to pick from, where the viewer is in none. */
  jobs: ViewerNotesHost["jobs"] | undefined;
  job: string | undefined;
  sending: boolean;
  onText: (text: string) => void;
  onHold: () => void;
  onSend: () => void;
  onPick: (job: string) => void;
  onRemove: (at: number) => void;
  onClose: () => void;
}) {
  const card = useRef<HTMLDivElement>(null);

  // Under what it is about, or over it where there is no room under, so the note never hides it; at the point where
  // neither has room. Kept inside the window, as a menu opened at a point is.
  useLayoutEffect(() => {
    const el = card.current;
    if (el === null) return;
    const gap = 8;
    const size = el.getBoundingClientRect();
    const fits = (top: number) => top >= gap && top + size.height <= window.innerHeight - gap;
    const top =
      anchor === undefined
        ? at.y
        : fits(anchor.bottom + gap)
          ? anchor.bottom + gap
          : fits(anchor.top - gap - size.height)
            ? anchor.top - gap - size.height
            : at.y;
    const left = anchor === undefined ? at.x : anchor.left;
    el.style.left = `${String(Math.max(gap, Math.min(left, window.innerWidth - size.width - gap)))}px`;
    el.style.top = `${String(Math.max(gap, Math.min(top, window.innerHeight - size.height - gap)))}px`;
  });

  // Focus comes in as it opens: onto the note, or onto Send where there is nothing to write.
  useLayoutEffect(() => {
    const el = card.current;
    if (el === null) return;
    (el.querySelector<HTMLElement>("textarea") ?? el.querySelector<HTMLElement>("button[data-send]") ?? el.querySelector<HTMLElement>("button"))?.focus();
  }, [about]);

  // A press outside closes it, as a popover does. The words stay kept for the next open.
  useLayoutEffect(() => {
    const down = (event: PointerEvent) => {
      const hit = event.target as Node;
      if (card.current?.contains(hit) === true) return;
      if (hit instanceof Element && hit.closest(".armada-dropdown-menu__panel") !== null) return;
      onClose();
    };
    window.addEventListener("pointerdown", down);
    return () => window.removeEventListener("pointerdown", down);
  }, [onClose]);

  const picked = jobs?.find((one) => one.id === job);
  const canSend = job !== undefined && !sending && (held.length > 0 || (about !== undefined && text.trim() !== ""));

  return (
    <div
      ref={card}
      className="armada-viewer-note"
      role="dialog"
      aria-label={about === undefined ? "Notes" : `Note on ${aboutSaid(about)}`}
      onKeyDown={(event) => {
        // The viewer's own keys stay out of the box.
        event.stopPropagation();
        if (event.key === "Escape") {
          // The Job picker's own menu takes its Esc first.
          if (event.target instanceof Element && event.target.closest("[role=menu]") !== null) return;
          event.preventDefault();
          onClose();
        } else if (sends(event)) {
          event.preventDefault();
          onSend();
        }
      }}
    >
      {held.length === 0 ? null : (
        <ol className="armada-viewer-note__held" aria-label="Held notes">
          {held.map((one, index) => (
            <li key={`${String(index)}:${one.text}`} className="armada-viewer-note__row">
              <NoteNumber n={index + 1} />
              <span className="armada-viewer-note__about">{aboutSaid(one.about)}</span>
              <span className="armada-viewer-note__text">{one.text}</span>
              <Tooltip label="Remove">
                <Button variant="ghost" size="sm" iconOnly aria-label={`Remove note ${String(index + 1)}`} onClick={() => onRemove(index)}>
                  <X size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            </li>
          ))}
        </ol>
      )}
      <div className="armada-viewer-note__head">
        {about === undefined ? null : (
          <>
            <NoteNumber n={held.length + 1} writing />
            <span className="armada-viewer-note__about">{aboutSaid(about)}</span>
          </>
        )}
        {jobs === undefined ? null : (
          <span className="armada-viewer-note__job">
            <DropdownMenu
              triggerLabel={picked?.title ?? "Job"}
              triggerTooltip="Send to"
              triggerAriaLabel={picked === undefined ? "Send to a Job" : `Send to ${picked.title}`}
              entries={jobs.map((one) => ({
                kind: "item" as const,
                id: one.id,
                label: one.title,
                selected: one.id === job,
              }))}
              onSelect={onPick}
            />
          </span>
        )}
      </div>
      {about === undefined ? null : (
        <Textarea
          aria-label="Note"
          placeholder="Note"
          rows={2}
          keep={keepKey}
          value={text}
          onChange={(event) => onText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && !event.shiftKey && !event.metaKey && !event.ctrlKey) {
              event.preventDefault();
              onHold();
            }
          }}
        />
      )}
      <div className="armada-viewer-note__acts">
        {about === undefined ? null : (
          <Button variant="secondary" size="sm" disabled={text.trim() === ""} onClick={onHold}>
            Hold
            <Kbd>↵</Kbd>
          </Button>
        )}
        <Button variant="primary" size="sm" data-send disabled={!canSend} onClick={onSend}>
          Send
          <KbdBinding binding="⌘↵" />
        </Button>
      </div>
    </div>
  );
}

import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type ReactNode,
} from "react";
import {
  BadgeCheck,
  FileText,
  ListTodo,
  MessageSquareText,
  PencilRuler,
  Reply,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { KbdBinding } from "../../primitives/Kbd/Kbd";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { forgetKept } from "../../keep";
import { pressesChord } from "../../keymap";

/** A glyph and the words its tooltip names it by. */
export type ReviewMark = {
  Glyph: LucideIcon;
  said: string;
};

/**
 * The marks of what a ledger row does not already name: a plan, a sketch, a draft and a document. A question, a walk and the rest
 * are marked by `waitingMark`, so the band and the sheet read one glyph for one thing. The plan is `list-todo`, the Record's plan
 * mark; the sketch is `pencil-ruler`, the ledger's sketch row; a draft is a proposal, `message-square-text`, which names a proposal
 * on a workflow step; a document is `file-text`, the glyph its artifact row draws.
 */
export const REVIEW_MARKS = {
  plan: {
    Glyph: ListTodo,
    said: "Plan",
  },
  sketch: {
    Glyph: PencilRuler,
    said: "Sketch",
  },
  draft: {
    Glyph: MessageSquareText,
    said: "Draft",
  },
  document: {
    Glyph: FileText,
    said: "Document",
  },
} as const satisfies Record<string, ReviewMark>;

export type ReviewItem = {
  id: string;
  /** What the row's first glyph is and its tooltip names. */
  mark: ReviewMark;
  name: string;
  /** The row's own menu, opened by a right press or the menu key: its entries and what choosing one does. */
  menu?: {
    entries: readonly DropdownMenuEntry[];
    onSelect: (entry: string) => void;
  };
  /** Answered and sent back stay in the list, muted and marked. */
  state: "open" | "answered" | "sent_back";
  /** The tooltip on an answered row's mark: how it was answered. Absent reads `Answered`. */
  said?: string;
  /**
   * What the bar offers while the item is open: the verb, which the primary button names the item with, and whether
   * Send back is there. **Absent when the body carries its own answers**, as a question's options do.
   */
  answer?: {
    verb: string;
    /** What the primary names after the verb, where the item's own name would not read. */
    label?: string;
    sendBack: boolean;
  };
  /** The body has not arrived: its shape is drawn in its place. */
  loading?: boolean;
  body: ReactNode;
};

/** The first open item after `from`, or else the first open one. `undefined` once every item is answered. */
export function nextOpenItem(items: readonly ReviewItem[], from: string): string | undefined {
  const here = items.findIndex((one) => one.id === from);
  return (
    items.slice(here + 1).find((one) => one.state === "open")?.id ??
    items.find((one) => one.state === "open")?.id
  );
}

const SEND_BACK = {
  ctrl: false,
  alt: true,
  shift: false,
  meta: false,
  key: "r",
} as const;
const PRIMARY = {
  ctrl: false,
  alt: false,
  shift: false,
  meta: true,
  key: "Enter",
} as const;

const STOPS =
  'button:not([disabled]), [href], input:not([disabled]), textarea:not([disabled]), select:not([disabled]), iframe, [tabindex]:not([tabindex="-1"])';

/**
 * Everything one Job has waiting on the person, in one sheet: a list of the items, oldest first, and the selected one drawn
 * beside it with its answers. **The head names the Job and carries the answers at its trailing end, in fixed places**: Send back,
 * then the primary, which names the item it covers (`Approve Plan v2`), then the close. Nothing approves more than one item.
 *
 * The host decides what each answer does; this part marks the row, brings the next open item up after an answer, and closes
 * after the last. `⌘↵` is the primary, `⌥R` Send back, which swaps the body's foot for a note that keeps what was typed under
 * `keep`; `Esc` leaves the note first and then closes. `↑` and `↓` move the list and draw the item at once. Tab stays inside.
 */
export function ReviewSheet({
  open,
  job,
  items,
  keep,
  onPrimary,
  onSendBack,
  onSelect,
  onClose,
  floor = false,
}: {
  open: boolean;
  /** The Job's name, which the sheet's head carries. */
  job: string;
  items: readonly ReviewItem[];
  /** What the notes are kept under, so two Jobs' drafts never meet. */
  keep: string;
  /** The primary act of an item: Approve, Run. */
  onPrimary: (id: string) => void;
  onSendBack: (id: string, note: string) => void;
  /** An item came up, by a key or a press or after an answer. */
  onSelect?: (id: string) => void;
  onClose: () => void;
  floor?: boolean;
}) {
  const first = () => items.find((one) => one.state === "open")?.id ?? items[0]?.id;
  const [at, setAt] = useState(first);
  const [noting, setNoting] = useState(false);
  // A row's own menu, drawn at the press or under the row for the menu key; closing it gives focus back to the row.
  const [menu, setMenu] = useState<{
    id: string;
    x: number;
    y: number;
  }>();
  const [wasOpen, setWasOpen] = useState(open);
  if (wasOpen !== open) {
    setWasOpen(open);
    if (open) {
      setAt(first());
      setNoting(false);
    }
  }
  const current = items.find((one) => one.id === at);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLDivElement>(null);

  const row = (id: string | undefined) =>
    id === undefined
      ? null
      : (list.current?.querySelector<HTMLElement>(`[data-item="${CSS.escape(id)}"]`) ?? null);
  const focusRow = (id: string | undefined) =>
    row(id)?.focus({
      preventScroll: true,
    });
  const choose = (id: string) => {
    setAt(id);
    setNoting(false);
  };

  // The list takes focus as the sheet opens. Its own effect runs after the sheet's, which puts focus on the close.
  useEffect(() => {
    if (open) focusRow(at);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    open,
  ]);

  useEffect(() => {
    if (open && at !== undefined) onSelect?.(at);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    open,
    at,
  ]);

  // An item that was open and is not any more has been answered: the next open one comes up, and the last answer closes the sheet.
  const before = useRef(new Map<string, ReviewItem["state"]>());
  useLayoutEffect(() => {
    const was = at === undefined ? undefined : before.current.get(at);
    before.current = new Map(
      items.map((one) => [
        one.id,
        one.state,
      ]),
    );
    if (!open || at === undefined || was !== "open" || current === undefined) return;
    if (current.state === "open") return;
    setNoting(false);
    const next = nextOpenItem(items, at);
    if (next === undefined) {
      onClose();
      return;
    }
    setAt(next);
    window.requestAnimationFrame(() => focusRow(next));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    items,
  ]);

  const answers = current?.state === "open" ? current.answer : undefined;
  const canNote = answers?.sendBack === true;

  const openNote = () => {
    if (!canNote) return;
    if (noting)
      root.current?.querySelector<HTMLTextAreaElement>("textarea[aria-label='Note']")?.focus();
    else setNoting(true);
  };

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      // A menu drawn over the sheet takes its own keys.
      if (document.querySelector('[role="menu"]')) return;
      if (event.key === "Tab") {
        const dialog = root.current?.closest<HTMLElement>('[role="dialog"]');
        if (dialog === null || dialog === undefined) return;
        const stops = [
          ...dialog.querySelectorAll<HTMLElement>(STOPS),
        ].filter((one) => one.getClientRects().length > 0);
        const start = stops[0];
        const end = stops[stops.length - 1];
        if (start === undefined || end === undefined) return;
        const active = document.activeElement;
        if (!dialog.contains(active)) {
          event.preventDefault();
          start.focus();
        } else if (event.shiftKey && active === start) {
          event.preventDefault();
          end.focus();
        } else if (!event.shiftKey && active === end) {
          event.preventDefault();
          start.focus();
        }
        return;
      }
      if (pressesChord(PRIMARY, event)) {
        if (answers === undefined || noting || current === undefined) return;
        event.preventDefault();
        onPrimary(current.id);
      } else if (pressesChord(SEND_BACK, event)) {
        if (!canNote) return;
        event.preventDefault();
        openNote();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const menuOf = menu === undefined ? undefined : items.find((one) => one.id === menu.id)?.menu;
  const move = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
      const target =
        event.target instanceof HTMLElement
          ? event.target.closest<HTMLElement>("[data-item]")
          : null;
      const one = items.find((it) => it.id === target?.dataset.item);
      if (target === null || one?.menu === undefined) return;
      event.preventDefault();
      const box = target.getBoundingClientRect();
      setMenu({
        id: one.id,
        x: box.left + box.width / 2,
        y: box.bottom,
      });
      return;
    }
    const keys = [
      "ArrowDown",
      "ArrowUp",
      "Home",
      "End",
    ];
    if (!keys.includes(event.key) || event.altKey || event.metaKey || event.ctrlKey) return;
    event.preventDefault();
    const here = items.findIndex((one) => one.id === at);
    const to =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? items.length - 1
          : Math.min(items.length - 1, Math.max(0, here + (event.key === "ArrowDown" ? 1 : -1)));
    const next = items[to];
    if (next === undefined) return;
    choose(next.id);
    focusRow(next.id);
  };

  return (
    <Sheet
      kind="job-review"
      open={open}
      floating
      size="widest"
      floor={floor}
      title={job}
      closeBinding="Esc"
      controls={
        <>
          {answers?.sendBack === true ? (
            <Button size="sm" variant="secondary" disabled={noting} onClick={openNote}>
              Send back
              <KbdBinding binding="⌥R" />
            </Button>
          ) : null}
          {answers === undefined || current === undefined ? null : (
            <Button
              size="sm"
              variant="primary"
              disabled={noting}
              onClick={() => onPrimary(current.id)}
            >
              {`${answers.verb} ${answers.label ?? current.name}`}
              <KbdBinding binding="⌘↵" />
            </Button>
          )}
        </>
      }
      bleed
      onClose={onClose}
    >
      <div ref={root} className="armada-job-review">
        <div
          ref={list}
          className="armada-job-review__list"
          role="group"
          aria-label="Waiting on you"
          onKeyDown={move}
        >
          {items.map((one) => {
            const kind = one.mark;
            const mark =
              one.state === "answered"
                ? {
                    Glyph: BadgeCheck,
                    said: one.said ?? "Answered",
                  }
                : one.state === "sent_back"
                  ? {
                      Glyph: Reply,
                      said: "Sent back",
                    }
                  : undefined;
            return (
              <button
                key={one.id}
                type="button"
                className="armada-job-review__item"
                data-item={one.id}
                data-on={one.id === at || undefined}
                data-state={one.state}
                aria-current={one.id === at ? "true" : undefined}
                tabIndex={one.id === at ? 0 : -1}
                onClick={() => choose(one.id)}
                onContextMenu={
                  one.menu === undefined
                    ? undefined
                    : (event) => {
                        event.preventDefault();
                        choose(one.id);
                        setMenu({
                          id: one.id,
                          x: event.clientX,
                          y: event.clientY,
                        });
                      }
                }
              >
                <Tooltip label={kind.said}>
                  <span className="armada-job-review__kind" role="img" aria-label={kind.said}>
                    <kind.Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                <span className="armada-job-review__name">{one.name}</span>
                {mark === undefined ? null : (
                  <Tooltip label={mark.said}>
                    <span className="armada-job-review__mark" role="img" aria-label={mark.said}>
                      <mark.Glyph size={12} strokeWidth={2} aria-hidden />
                    </span>
                  </Tooltip>
                )}
              </button>
            );
          })}
        </div>
        {current === undefined ? null : (
          <section key={current.id} className="armada-job-review__detail" aria-label={current.name}>
            <div className="armada-job-review__body">
              {current.loading === true ? (
                <div className="armada-job-review__loading">
                  <Skeleton width="40%" />
                  <Skeleton />
                  <Skeleton width="85%" />
                  <Skeleton width="60%" />
                </div>
              ) : (
                current.body
              )}
            </div>
            {noting && canNote ? (
              <ReviewNote
                noteKey={`${keep}:${current.id}:note`}
                onSend={(note) => onSendBack(current.id, note)}
                onLeave={() => {
                  setNoting(false);
                  focusRow(current.id);
                }}
              />
            ) : null}
          </section>
        )}
      </div>
      {menu === undefined || menuOf === undefined ? null : (
        <DropdownMenu
          triggerLabel="Item"
          panelClass="armada-job-review__menu"
          at={{
            x: menu.x,
            y: menu.y,
          }}
          open
          entries={[
            ...menuOf.entries,
          ]}
          onOpenChange={(next) => {
            if (next) return;
            setMenu(undefined);
            focusRow(menu.id);
          }}
          onSelect={menuOf.onSelect}
        />
      )}
    </Sheet>
  );
}

/** The note that sends an item back, in the body's foot. `Esc` puts it away and keeps what was typed. */
function ReviewNote({
  noteKey,
  onSend,
  onLeave,
}: {
  noteKey: string;
  onSend: (note: string) => void;
  onLeave: () => void;
}) {
  const [note, setNote] = useState("");
  const send = () => {
    if (note.trim() === "") return;
    forgetKept(noteKey);
    onSend(note.trim());
  };
  return (
    <div className="armada-job-review__foot" data-keeps-esc="">
      <Textarea
        aria-label="Note"
        rows={3}
        autoFocus
        keep={noteKey}
        value={note}
        onChange={(event) => setNote(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onLeave();
          } else if (pressesChord(PRIMARY, event.nativeEvent)) {
            event.preventDefault();
            send();
          }
        }}
      />
      <div className="armada-job-review__foot-acts">
        <Button size="sm" variant="primary" disabled={note.trim() === ""} onClick={send}>
          Send back
          <KbdBinding binding="⌘↵" />
        </Button>
        <Button size="sm" variant="ghost" onClick={onLeave}>
          Cancel
          <KbdBinding binding="Esc" />
        </Button>
      </div>
    </div>
  );
}

import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import { isPressed, pressedSlot } from "../../keymap";
import { Readout } from "../../primitives/Readout/Readout";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ageOf } from "../../time-format";
import { LessonCard, type LessonAnswers, type LessonEvidence, type LessonSettled, type RetroCite } from "../LessonCard/LessonCard";
import { LandsMark, type Lands } from "../LandsMark/LandsMark";
import { WhoMark, type Who } from "../WhoMark/WhoMark";

/** One item of one Session's retro, as the Lessons page lists it. */
export type LessonRow = {
  /** Unique within the list. */
  id: string;
  sessionId: string;
  who: Who;
  /** Where its fix lands. Absent on an item stored before that was written. */
  landsIn?: Lands;
  statement: string;
  title?: string;
  what?: string;
  fix?: string;
  /** The Session, as a person reads it. Its id rides the label's tooltip. */
  session: string;
  sessionExact?: string;
  /** When its retro was written, an RFC 3339 instant. */
  at: string;
  /** The answers this row offers, absent where it is already answered. */
  answers?: LessonAnswers;
  /** What an answered row reads as while it stays on screen. */
  settled?: LessonSettled;
  cites?: readonly RetroCite[];
  /** Set while the rows it cites are not read yet: the control that reads them. */
  evidence?: LessonEvidence;
};

export type LessonListProps = {
  /** Newest retro first, as Fleet serves them. */
  rows: readonly LessonRow[];
  /** A press on the Session a row came from opens its retro. The row's id says which, where a Session has several. */
  onOpen: (sessionId: string, rowId: string) => void;
};

/**
 * What got in the way across Sessions, newest first — the Retros page's one list,
 * each item a `LessonCard` the owner can agree or disagree with.
 *
 * **No heading and no count.** A count beside the rows it counts is
 * design-system hard rule 7, and an empty list draws nothing.
 *
 * **Not virtualized, and bounded.** `list_lessons` answers 200 at most where
 * `most` is not sent, and Bridge sends none.
 */
export function LessonList({ rows, onOpen }: LessonListProps) {
  if (rows.length === 0) return null;
  return (
    <ul className="armada-lessons" aria-label="Retros">
      {rows.map((row) => (
        <RowCard key={row.id} row={row} onOpen={onOpen} />
      ))}
    </ul>
  );
}

const optionId = (id: string) => `retro-item-${id}`;

/**
 * The retro items as the Logbook's Retros tab draws them: one line an item on the left, and the item open on the right
 * as its `LessonCard`. The rows take the Logbook's own row look, so the two tabs read alike. The arrows move, `Enter`
 * goes to the item and `Esc` comes back.
 */
export function LessonSplit({ rows, onOpen }: LessonListProps) {
  const list = useRef<HTMLDivElement>(null);
  const detail = useRef<HTMLElement>(null);
  const [picked, setPicked] = useState<string>();
  const open = rows.find((row) => row.id === picked) ?? rows[0];
  const now = Date.now();

  useEffect(() => {
    if (open !== undefined) document.getElementById(optionId(open.id))?.scrollIntoView({ block: "nearest" });
  }, [open?.id]);

  if (rows.length === 0) return null;

  function pressed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement;
    const inList = list.current?.contains(target) === true;
    const slot = pressedSlot("move_focus", event);
    if (slot !== -1 && inList) {
      event.preventDefault();
      const at = rows.findIndex((row) => row.id === open?.id);
      const next = rows[Math.min(rows.length - 1, Math.max(0, at + (slot === 0 || slot === 2 ? 1 : -1)))];
      if (next !== undefined) setPicked(next.id);
    } else if (isPressed("open_focused", event) && inList) {
      event.preventDefault();
      detail.current?.focus();
    } else if (isPressed("close", event) && detail.current?.contains(target) === true) {
      event.preventDefault();
      list.current?.focus();
    }
  }

  return (
    <div className="armada-lessons-split" onKeyDown={pressed}>
      <div className="armada-lessons-split__list armada-logbook__panel">
        <div
          ref={list}
          role="listbox"
          aria-label="Retros"
          tabIndex={0}
          className="armada-lessons-split__rows"
          aria-activedescendant={open === undefined ? undefined : optionId(open.id)}
        >
          {rows.map((row) => (
            <div
              key={row.id}
              id={optionId(row.id)}
              role="option"
              aria-selected={row.id === open?.id}
              className="armada-logbook__row"
              onClick={() => {
                setPicked(row.id);
                list.current?.focus();
              }}
            >
              <span className="armada-logbook__text">{row.title ?? row.statement}</span>
              <span className="armada-logbook__marks">
                <WhoMark who={row.who} />
                {row.landsIn === undefined ? null : <LandsMark lands={row.landsIn} />}
              </span>
              <Tooltip label={new Date(row.at).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })}>
                <Readout fixed>
                  <time dateTime={row.at}>{ageOf(Date.parse(row.at), now)}</time>
                </Readout>
              </Tooltip>
            </div>
          ))}
        </div>
      </div>
      <section ref={detail} className="armada-lessons-split__detail" aria-label="Retro item" tabIndex={-1}>
        {open === undefined ? null : (
          <ul className="armada-lessons">
            <RowCard key={open.id} row={open} onOpen={onOpen} />
          </ul>
        )}
      </section>
    </div>
  );
}

function RowCard({ row, onOpen }: { row: LessonRow; onOpen: LessonListProps["onOpen"] }) {
  return (
        <LessonCard
          item={{
            who: row.who,
            ...(row.landsIn === undefined ? {} : { landsIn: row.landsIn }),
            statement: row.statement,
            ...(row.title === undefined ? {} : { title: row.title }),
            ...(row.what === undefined ? {} : { what: row.what }),
            ...(row.fix === undefined ? {} : { fix: row.fix }),
            ...(row.cites === undefined ? {} : { cites: row.cites }),
          }}
          {...(row.evidence === undefined ? {} : { evidence: row.evidence })}
          {...(row.answers === undefined ? {} : { answers: row.answers })}
          {...(row.settled === undefined ? {} : { settled: row.settled })}
          from={{
            label: row.session,
            ...(row.sessionExact === undefined ? {} : { exact: row.sessionExact }),
            onOpen: () => onOpen(row.sessionId, row.id),
          }}
        />
  );
}

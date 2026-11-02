import { MoveRight } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

import { isPressed, pressedSlot } from "../../keymap";
import { Table, TableBody, TableCell, TableRow } from "../../primitives/Table/Table";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ageOf } from "../../time-format";
import {
  LessonCard,
  type LessonAnswers,
  type LessonEvidence,
  type LessonSettled,
  type RetroCite,
} from "../LessonCard/LessonCard";
import { ListCard, ListCardMark } from "../ListCard/ListCard";
import { LandsMark, landsWord, type Lands } from "../LandsMark/LandsMark";
import { WhoMark, whoWord, type Who } from "../WhoMark/WhoMark";

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

export type LessonSplitProps = LessonListProps & {
  /** What the list's strip holds, the places and the acts of the page. */
  strip?: ReactNode;
};

/**
 * The retro items as the Logbook's Retros tab draws them, as its entries are: on the left a list of them, a card each with its
 * route, its title, its Job and its age, and on the right the open one under its own head, its card and its record under it.
 * The arrows move, `Enter` goes to the item and `Esc` comes back. The frames are drawn with no items, for the strip they hold.
 */
export function LessonSplit({ rows, onOpen, strip }: LessonSplitProps) {
  const list = useRef<HTMLDivElement>(null);
  const detail = useRef<HTMLElement>(null);
  const [picked, setPicked] = useState<string>();
  // The head's own place for the open item's answers, which the card draws into it.
  const [acts, setActs] = useState<HTMLElement | null>(null);
  const open = rows.find((row) => row.id === picked) ?? rows[0];
  const now = Date.now();

  useEffect(() => {
    if (open !== undefined)
      document.getElementById(optionId(open.id))?.scrollIntoView({
        block: "nearest",
      });
  }, [
    open?.id,
  ]);

  function pressed(event: KeyboardEvent<HTMLDivElement>): void {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const target = event.target as HTMLElement;
    const inList = list.current?.contains(target) === true;
    const slot = pressedSlot("move_focus", event);
    if (slot !== -1 && inList) {
      event.preventDefault();
      const at = rows.findIndex((row) => row.id === open?.id);
      const next =
        rows[Math.min(rows.length - 1, Math.max(0, at + (slot === 0 || slot === 2 ? 1 : -1)))];
      if (next !== undefined) setPicked(next.id);
    } else if (isPressed("open_focused", event) && inList) {
      event.preventDefault();
      detail.current?.focus();
    } else if (isPressed("close", event) && detail.current?.contains(target) === true) {
      event.preventDefault();
      list.current?.focus();
    }
  }

  const headline = open === undefined ? "" : (open.title ?? open.statement);
  return (
    <div className="armada-lessons-split" onKeyDown={pressed}>
      <div className="armada-logbook__panel armada-logbook__log armada-lessons-split__list">
        <div className="armada-logbook__band armada-logbook__strip">{strip}</div>
        {rows.length === 0 ? null : (
          <div
            ref={list}
            role="grid"
            aria-label="Retros"
            tabIndex={0}
            className="armada-logbook__table"
            aria-activedescendant={open === undefined ? undefined : optionId(open.id)}
          >
            <div className="armada-logbook__group" role="rowgroup">
              {rows.map((row) => (
                <ListCard
                  key={row.id}
                  id={optionId(row.id)}
                  role="row"
                  tabIndex={-1}
                  aria-selected={row.id === open?.id}
                  selected={row.id === open?.id}
                  className="armada-logbook__row"
                  aria-label={row.title ?? row.statement}
                  onClick={() => {
                    setPicked(row.id);
                    list.current?.focus();
                  }}
                  title={row.title ?? row.statement}
                  marks={
                    <>
                      <ListCardMark>
                        <WhoMark who={row.who} />
                      </ListCardMark>
                      <ListCardMark>
                        <MoveRight size={10} strokeWidth={2} aria-hidden />
                      </ListCardMark>
                      <ListCardMark>
                        {row.landsIn === undefined ? null : <LandsMark lands={row.landsIn} />}
                      </ListCardMark>
                    </>
                  }
                  tag={
                    <Tooltip label={row.sessionExact ?? row.session}>
                      <span className="armada-lessons-split__job">{row.session}</span>
                    </Tooltip>
                  }
                  at={
                    <Tooltip label={dated(row.at)}>
                      <time dateTime={row.at}>{ageOf(Date.parse(row.at), now)}</time>
                    </Tooltip>
                  }
                />
              ))}
            </div>
          </div>
        )}
      </div>
      <section
        ref={detail}
        className="armada-logbook__detail armada-lessons-split__detail"
        aria-label="Retro item"
        tabIndex={-1}
      >
        {open === undefined ? (
          <div className="armada-logbook__card" />
        ) : (
          <div className="armada-logbook__card">
            <div className="armada-logbook__band armada-logbook__head">
              <Tooltip label={headline} asChild>
                <h2 className="armada-logbook__head-title">{headline}</h2>
              </Tooltip>
              <span ref={setActs} className="armada-logbook__head-acts" />
            </div>
            <div className="armada-logbook__body">
              <div className="armada-lessons-split__card">
                <ul className="armada-lessons">
                  <RowCard key={open.id} row={open} onOpen={onOpen} actsIn={acts} />
                </ul>
              </div>
              <div className="armada-logbook__part">
                <div className="armada-logbook__band armada-logbook__legend">Record</div>
                <Table className="armada-logbook__record" aria-label="Record">
                  <colgroup>
                    <col className="armada-logbook__col-by" />
                    <col />
                  </colgroup>
                  <TableBody>
                    <Fact name="Got in the way of">{whoWord(open.who)}</Fact>
                    <Fact name="Fix lands in">
                      {open.landsIn === undefined ? "Not said" : landsWord(open.landsIn)}
                    </Fact>
                    <Fact name="Job">
                      <button
                        type="button"
                        className="armada-logbook__job"
                        onClick={() => onOpen(open.sessionId, open.id)}
                      >
                        {open.session}
                      </button>
                    </Fact>
                    <Fact name="Written" mono>
                      <Tooltip label={dated(open.at)}>
                        <time dateTime={open.at}>{ageOf(Date.parse(open.at), now)}</time>
                      </Tooltip>
                    </Fact>
                  </TableBody>
                </Table>
              </div>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}

const dated = (at: string): string =>
  new Date(at).toLocaleString(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  });

/** One row of the record: its name, and its value. */
function Fact({
  name,
  mono = false,
  children,
}: {
  name: string;
  mono?: boolean;
  children: ReactNode;
}) {
  return (
    <TableRow>
      <th scope="row" className="armada-logbook__fact">
        {name}
      </th>
      <TableCell variant={mono ? "mono" : "primary"}>{children}</TableCell>
    </TableRow>
  );
}

function RowCard({
  row,
  onOpen,
  actsIn,
}: {
  row: LessonRow;
  onOpen: LessonListProps["onOpen"];
  actsIn?: HTMLElement | null;
}) {
  return (
    <LessonCard
      item={{
        who: row.who,
        ...(row.landsIn === undefined
          ? {}
          : {
              landsIn: row.landsIn,
            }),
        statement: row.statement,
        ...(row.title === undefined
          ? {}
          : {
              title: row.title,
            }),
        ...(row.what === undefined
          ? {}
          : {
              what: row.what,
            }),
        ...(row.fix === undefined
          ? {}
          : {
              fix: row.fix,
            }),
        ...(row.cites === undefined
          ? {}
          : {
              cites: row.cites,
            }),
      }}
      {...(row.evidence === undefined
        ? {}
        : {
            evidence: row.evidence,
          })}
      {...(row.answers === undefined
        ? {}
        : {
            answers: row.answers,
          })}
      {...(row.settled === undefined
        ? {}
        : {
            settled: row.settled,
          })}
      {...(actsIn === undefined
        ? {}
        : {
            actsIn,
          })}
      from={{
        label: row.session,
        ...(row.sessionExact === undefined
          ? {}
          : {
              exact: row.sessionExact,
            }),
        onOpen: () => onOpen(row.sessionId, row.id),
      }}
    />
  );
}

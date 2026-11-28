// A run of tool calls as one card, ported from the native Bridge's (`crates/bridge-native/src/job_view/runs.rs`, take A,
// approved 26 Nov). Live: the newest thought as the headline, one tile per call, and the running call in a box under
// them with its timer, Stop and the lines it wrote. Done: the tiles, the run's words and its time on one line. A tap
// anywhere on the head opens the thoughts and calls in order; a tile is no target of its own, since a phone has no hover.

import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight, Globe, LoaderCircle, Send, Smartphone, Wrench } from "lucide-react";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { useNowWhile } from "../../clock";
import { elapsedWords } from "../SessionRetroRow/SessionRetroRow";
import { useTeachMenu } from "../TeachPanel/TeachMenu";
import { SessionStop } from "./SessionStop";
import { callLine, callsLine, kindOf, workingOn, type CallKind } from "./calls-line";
import type { SessionThreadRow, TeachPick } from "./SessionThread";
import "./CallsCard.css";

type RunRow = Extract<
  SessionThreadRow,
  {
    kind: "tool" | "reasoning";
  }
>;
type ToolRow = Extract<
  RunRow,
  {
    kind: "tool";
  }
>;

/** One call as the strip and the open card read it. */
type Seen = {
  row: ToolRow;
  kind: CallKind;
  phrase: string;
  /** Drawn after the phrase: the Job a message went to. */
  peer?: string;
  /** A second line that stays: a message's summary. */
  note?: string;
  running: boolean;
  /** When it ended: the next row, else the run's end. Absent while it runs. */
  end?: string;
};

const took = (from: string, to: string | number | undefined): string => {
  if (to === undefined) return "";
  const ms = (typeof to === "number" ? to : Date.parse(to)) - Date.parse(from);
  return Number.isNaN(ms) ? "" : elapsedWords(ms);
};

/** The Job a message went to, by title else name. */
const peerOf = (message: NonNullable<ToolRow["message"]>): string =>
  message.to.title?.trim() || message.to.name;

/** A message's second line: its summary, else the first line of its words. */
const noteOf = (message: NonNullable<ToolRow["message"]>): string | undefined =>
  message.summary?.trim() || message.text.split("\n").find((line) => line.trim() !== "");

/** The glyphs a finished call's tile draws, each the native icon's last frame in its 24-unit box. */
function Glyph({ kind }: { kind: CallKind }): ReactNode {
  switch (kind) {
    case "read":
      return (
        <svg viewBox="0 0 24 24" aria-hidden>
          <path d="M6 3h8l4 4v14H6z" />
          <path d="M14 3v4h4" />
          <path d="M8.5 10h7M8.5 13h7M8.5 16h5" strokeWidth={1.6} />
        </svg>
      );
    case "run":
      return (
        <svg viewBox="0 0 24 24" aria-hidden>
          <path d="M4 17l6-6-6-6" />
          <rect x="12" y="16" width="8" height="3.2" rx="0.8" className="armada-calls-card__fill" />
        </svg>
      );
    case "edit":
      return (
        <svg viewBox="0 0 24 24" aria-hidden>
          <path d="M3 20q1.5-2 3 0t3 0t3 0t3 0" />
          <path d="M15 20l.8-3.2 6.2-6.2 2.4 2.4-6.2 6.2z" strokeWidth={1.5} />
          <path d="M20.8 11.8l2.4 2.4" strokeWidth={1.5} />
        </svg>
      );
    case "search":
      return (
        <svg viewBox="0 0 24 24" aria-hidden>
          <path d="M3 6h18M3 12h18M3 18h12" strokeWidth={1.6} opacity={0.35} />
          <rect x="8" y="10" width="7" height="4" rx="1" className="armada-calls-card__fill" opacity={0.45} />
          <circle cx="11.5" cy="12" r="3.6" />
          <path d="M14.1 14.6l2.8 2.8" />
        </svg>
      );
    case "agent":
      return (
        <svg viewBox="0 0 24 24" aria-hidden>
          <rect x="4" y="9" width="16" height="11" rx="3" />
          <path d="M12 9V5.8" />
          <circle cx="12" cy="4.5" r="1.3" className="armada-calls-card__fill" />
          <circle cx="9" cy="14.5" r="1.4" className="armada-calls-card__fill" />
          <circle cx="15" cy="14.5" r="1.4" className="armada-calls-card__fill" />
        </svg>
      );
    case "web":
      return <Globe aria-hidden />;
    case "message":
      return <Send aria-hidden />;
    case "other":
      return <Wrench aria-hidden />;
  }
}

/** One call's tile: 20 square, its hue at 15% and its glyph in the hue lightened. The running call's is blue with a loader that turns. */
function Tile({ kind, running, said }: { kind: CallKind; running: boolean; said: string }) {
  return (
    <span
      className="armada-calls-card__tile"
      data-kind={kind}
      data-running={running ? "" : undefined}
      role="listitem"
      aria-label={said}
    >
      {running ? <LoaderCircle aria-hidden className="armada-calls-card__spin" /> : <Glyph kind={kind} />}
    </span>
  );
}

/** The time a call or a run took, counting each second while it runs. */
function Timer({ from, end, running }: { from: string; end?: string; running: boolean }) {
  const now = useNowWhile(running);
  return <span className="armada-calls-card__time">{took(from, running ? now : end)}</span>;
}

/** One call of an open run. Where Teach is offered it takes focus, and the menu opens on it. */
function CallRow({ one, onTeach }: { one: Seen; onTeach?: (pick: TeachPick) => void }) {
  const { on, menu } = useTeachMenu(
    "Command",
    onTeach === undefined
      ? undefined
      : () =>
          onTeach({
            kind: "command",
            text: one.row.text,
            at: one.row.at,
          }),
  );
  const said = one.peer === undefined ? one.phrase : `${one.phrase} ${one.peer}`;
  return (
    <li
      className="armada-calls-card__call"
      aria-label={said}
      {...(onTeach === undefined
        ? {}
        : {
            tabIndex: 0,
          })}
      {...on}
    >
      <span className="armada-calls-card__line">
        <Tile kind={one.kind} running={one.running} said={one.phrase} />
        <span className="armada-calls-card__phrase">
          {one.phrase}
          {one.peer === undefined ? null : (
            <span className="armada-calls-card__peer"> {one.peer}</span>
          )}
        </span>
        {one.row.fromPhone === true ? (
          <Tooltip label="From phone">
            <span className="armada-calls-card__door">
              <Smartphone aria-hidden />
            </span>
          </Tooltip>
        ) : null}
        <Timer from={one.row.at} {...(one.end === undefined ? {} : { end: one.end })} running={one.running} />
      </span>
      {one.note === undefined ? null : <span className="armada-calls-card__note">{one.note}</span>}
      {menu}
    </li>
  );
}

export function CallsCard({
  rows,
  live,
  end,
  tails,
  onStop,
  onTeach,
}: {
  rows: readonly RunRow[];
  /** The run the agent is in now. */
  live: boolean;
  /** When the run ended: the first row after it. */
  end?: string;
  /** The newest lines each background command wrote, by the id of the call that started it. */
  tails?: readonly {
    call: string;
    lines: readonly string[];
  }[];
  onStop?: () => void;
  onTeach?: (pick: TeachPick) => void;
}) {
  const [open, setOpen] = useState(false);
  const calls = rows.filter((row): row is ToolRow => row.kind === "tool");
  // A thought last in a live run means no call runs: the agent is thinking.
  const thinking = live && rows[rows.length - 1]?.kind === "reasoning";
  const running = live && !thinking ? calls[calls.length - 1] : undefined;
  const thoughts = rows.filter((row) => row.kind === "reasoning");
  const latest = thoughts[thoughts.length - 1];
  const seen = new Map<string, Seen>(
    calls.map((row) => {
      const at = rows.indexOf(row);
      const isRunning = row === running;
      const ends = rows[at + 1]?.at ?? end;
      const base = {
        row,
        running: isRunning,
        ...(isRunning || ends === undefined
          ? {}
          : {
              end: ends,
            }),
      };
      if (row.message !== undefined) {
        const note = noteOf(row.message);
        return [
          row.id,
          {
            ...base,
            kind: "message",
            phrase: "Message to",
            peer: peerOf(row.message),
            ...(note === undefined
              ? {}
              : {
                  note,
                }),
          },
        ];
      }
      return [
        row.id,
        {
          ...base,
          kind: kindOf(row.text),
          // The call's own phrase, where its input carried one; else the one made from its line.
          phrase: row.description ?? callLine(row, isRunning),
        },
      ];
    }),
  );
  const strip = calls.map((row) => {
    const one = seen.get(row.id)!;
    const phrase = one.peer === undefined ? one.phrase : `${one.phrase} ${one.peer}`;
    return <Tile key={row.id} kind={one.kind} running={one.running} said={phrase} />;
  });
  const first = rows[0]?.at;
  const label = `Tool calls: ${[
    ...new Set(calls.map((row) => row.text.split(/\s/, 1)[0] ?? "")),
  ]
    .filter((name) => name !== "")
    .join(", ")}`;
  const Chevron = open ? ChevronDown : ChevronRight;
  const toggle = () => setOpen((was) => !was);
  // What a live card's headline says: the newest thought, else the running call's phrase, else the last call's.
  const last = calls[calls.length - 1];
  const headline =
    latest?.text.split("\n", 1)[0] ??
    (running ?? last ? seen.get((running ?? last)!.id)!.phrase : "");
  const now = running === undefined ? undefined : seen.get(running.id)!;
  const written = running === undefined ? [] : (tails?.find((one) => one.call === running.id)?.lines ?? []);
  return (
    <li
      className="armada-calls-card"
      data-live={live ? "" : undefined}
      role="group"
      aria-label={label}
    >
      <button
        type="button"
        className="armada-calls-card__head"
        aria-expanded={open}
        aria-label={label}
        onClick={toggle}
      >
        {live ? (
          <>
            <span
              key={latest?.id ?? running?.id ?? "headline"}
              className="armada-calls-card__headline"
              data-thinking={thinking ? "" : undefined}
            >
              {headline}
            </span>
            {first === undefined ? null : <Timer from={first} running />}
          </>
        ) : (
          <>
            <span className="armada-calls-card__strip armada-calls-card__strip--one" role="list">
              {strip}
            </span>
            <span className="armada-calls-card__sum">{callsLine(calls, false)}</span>
            {first === undefined ? null : (
              <Timer from={first} running={false} end={end ?? rows[rows.length - 1]?.at ?? first} />
            )}
          </>
        )}
        <Chevron aria-hidden className="armada-calls-card__chevron" />
      </button>
      {live ? (
        <span className="armada-calls-card__strip" role="list" onClick={toggle}>
          {strip}
        </span>
      ) : null}
      {now === undefined ? null : (
        <div className="armada-calls-card__running" role="status" aria-label={workingOn(now.row.text) ?? now.phrase}>
          <span className="armada-calls-card__doing">
            <span className="armada-calls-card__command">{workingOn(now.row.text) ?? now.phrase}</span>
            <Timer from={now.row.at} running />
            {onStop === undefined ? null : <SessionStop onStop={onStop} />}
          </span>
          {written.map((line, at) => (
            <span key={at} className="armada-calls-card__output">
              {line}
            </span>
          ))}
        </div>
      )}
      {open ? (
        <ol className="armada-calls-card__open">
          {rows.map((row) =>
            row.kind === "reasoning" ? (
              <li key={row.id} className="armada-calls-card__thought">
                {row.text}
              </li>
            ) : (
              <CallRow
                key={row.id}
                one={seen.get(row.id)!}
                {...(onTeach === undefined
                  ? {}
                  : {
                      onTeach,
                    })}
              />
            ),
          )}
        </ol>
      ) : null}
    </li>
  );
}

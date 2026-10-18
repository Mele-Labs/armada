import { memo, useEffect, useState } from "react";
import { Rewind } from "lucide-react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { LessonRoute } from "../LessonCard/LessonCard";
import type { Lands } from "../LandsMark/LandsMark";
import type { Who } from "../WhoMark/WhoMark";

/** Where the writing of a Session's retro has got to. `retro` row, `stage`. */
export type RetroRowStage = "asking_agent" | "reading_record" | "writing" | "written" | "failed";

/** One item as it settles. **A field is absent until its value is complete**, so none is half a sentence. */
export type RetroRowItem = {
  title?: string;
  what?: string;
  fix?: string;
  who?: Who;
  landsIn?: Lands;
};

/** The `retro` row of a Session's thread, as the thread draws it. */
export type SessionRetroRowData = {
  id: string;
  at: string;
  kind: "retro";
  stage: RetroRowStage;
  /** When it reached this stage, in epoch milliseconds. Absent where the instant would not parse. */
  stageSince?: number;
  /** How much of the model's answer has arrived. */
  characters: number;
  items: readonly RetroRowItem[];
  /** The kept retro's number, on `written`. */
  retro?: number;
  /** Fleet's words for why it was not kept, on `failed`. */
  why?: string;
};

export type SessionRetroRowProps = {
  row: SessionRetroRowData;
  /** Opens the kept retro on the Retros page. Absent, there is no link. */
  onOpen?: (retro: number) => void;
  /** The instant the elapsed time is read against. Absent, the clock; a story fixes it. */
  now?: number;
};

const STAGE_SAID: Record<Exclude<RetroRowStage, "written" | "failed">, string> = {
  asking_agent: "Asking the agent what got in the way",
  reading_record: "Reading what Fleet kept of the Job",
  writing: "Writing the retro",
};

/** "7s", "1m 05s", "1h 02m": whole seconds up to a minute, then the unit above. */
export function elapsedWords(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  if (total < 60) return `${total}s`;
  const minutes = Math.floor(total / 60);
  if (minutes < 60) return `${minutes}m ${String(total % 60).padStart(2, "0")}s`;
  return `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

/** How many items, as a phrase: `1 item`, `4 items`. */
export function itemsWords(count: number): string {
  return count === 1 ? "1 item" : `${count} items`;
}

/**
 * Whether a redraw would change nothing. **By value, not by reference**: a window that hears the
 * thread over IPC receives every row as a new object each time any of them moves, so the row's own
 * reference says nothing, and a frame that is not this row's must not redraw its card.
 */
function sameProps(was: SessionRetroRowProps, now: SessionRetroRowProps): boolean {
  return was.onOpen === now.onOpen && was.now === now.now && (was.row === now.row || JSON.stringify(was.row) === JSON.stringify(now.row));
}

/** The clock, ticking once a second while `running`, unless a story fixed it. */
function useNow(running: boolean, fixed: number | undefined): number {
  const [now, setNow] = useState(() => fixed ?? Date.now());
  useEffect(() => {
    if (!running || fixed !== undefined) return;
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, [running, fixed]);
  return fixed ?? now;
}

/** Space held for a field that has not arrived, so the card does not jump as it fills. */
function Waiting({ size }: { size: "title" | "what" | "fix" | "route" }) {
  return <span className="armada-session-retro__waiting" data-size={size} aria-hidden />;
}

function Settling({ item, last }: { item: RetroRowItem; last: boolean }) {
  return (
    <li className="armada-session-retro__item" aria-label={item.title ?? "Item being written"} data-writing={last || undefined}>
      <div className="armada-session-retro__route">
        {item.who === undefined && item.landsIn === undefined && last ? <Waiting size="route" /> : <LessonRoute {...(item.who === undefined ? {} : { who: item.who })} {...(item.landsIn === undefined ? {} : { landsIn: item.landsIn })} />}
      </div>
      {item.title === undefined ? last ? <Waiting size="title" /> : null : <h3 className="armada-session-retro__title armada-session-retro__field">{item.title}</h3>}
      {item.what === undefined ? last ? <Waiting size="what" /> : null : <p className="armada-session-retro__what armada-session-retro__field">{item.what}</p>}
      {item.fix === undefined ? (
        last ? <Waiting size="fix" /> : null
      ) : (
        <div className="armada-session-retro__fix armada-session-retro__field">
          <span className="armada-session-retro__fix-label">What would change</span>
          <p className="armada-session-retro__fix-said">{item.fix}</p>
        </div>
      )}
      {last ? <span className="armada-session-retro__still">Still being written</span> : null}
    </li>
  );
}

/**
 * A Session's retro in its thread, from the press to the end. **One card that stays**: while Fleet
 * writes it the stage is said in a line with the time it has been at it, and during `writing` the
 * items settle in as cards, a field at a time, the last marked as still being written. When it is
 * written the card folds to one line, `Retro 3 written · 4 items`, with a link that opens the kept
 * retro and the items by title; when it fails Fleet's words are its alert.
 *
 * **Drawn from the latest row only**: Fleet replaces the row by id as it goes, so nothing here
 * reads an earlier frame, and the card is memoised on the row's value so a frame of another row leaves it alone.
 * The host passes `onOpen` stable, or the memo is spent. The space a field will fill is held while it is awaited, and under
 * reduced motion a field simply lands.
 */
export const SessionRetroRow = memo(function SessionRetroRow({ row, onOpen, now }: SessionRetroRowProps) {
  const running = row.stage !== "written" && row.stage !== "failed";
  const clock = useNow(running, now);
  const elapsed = running && row.stageSince !== undefined ? elapsedWords(clock - row.stageSince) : undefined;

  if (row.stage === "written") {
    const titled = row.items.map((one) => one.title ?? one.what).filter((one): one is string => one !== undefined && one !== "");
    const count = row.items.length;
    const retro = row.retro;
    return (
      <li className="armada-session-retro" role="region" aria-label="Retro" data-stage="written">
        <div className="armada-session-retro__head">
          <Rewind size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-retro__said">
            {retro === undefined ? "Retro written" : `Retro ${retro} written`}
            {count === 0 ? "" : ` · ${itemsWords(count)}`}
          </span>
          {retro === undefined || onOpen === undefined ? null : (
            <Button variant="ghost" size="sm" className="armada-session-retro__open" onClick={() => onOpen(retro)}>
              Open retro
            </Button>
          )}
        </div>
        {titled.length === 0 ? null : (
          <ul className="armada-session-retro__titles" aria-label="Items">
            {titled.map((title, at) => (
              <li key={at}>{title}</li>
            ))}
          </ul>
        )}
      </li>
    );
  }

  if (row.stage === "failed") {
    return (
      <li className="armada-session-retro" role="region" aria-label="Retro" data-stage="failed">
        <div className="armada-session-retro__head">
          <Rewind size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-retro__said">The retro was not written</span>
        </div>
        <Alert tone="escalated">{row.why === undefined || row.why === "" ? "Fleet gave no reason." : row.why}</Alert>
      </li>
    );
  }

  const writing = row.stage === "writing";
  return (
    <li className="armada-session-retro" role="region" aria-label="Retro" data-stage={row.stage}>
      <div className="armada-session-retro__head">
        <Rewind size={12} strokeWidth={2} aria-hidden className="armada-session-retro__mark" />
        <span className="armada-session-retro__said" role="status">
          {STAGE_SAID[row.stage]}
        </span>
        {elapsed === undefined ? null : (
          <span className="armada-session-retro__elapsed mono" role="timer" aria-label={`${elapsed} in this stage`}>
            {elapsed}
          </span>
        )}
        {writing && row.characters > 0 ? <span className="armada-session-retro__hint">{row.characters} characters</span> : null}
      </div>
      {writing && row.items.length > 0 ? (
        <ol className="armada-session-retro__items" aria-label="Items so far">
          {row.items.map((item, at) => (
            <Settling key={at} item={item} last={at === row.items.length - 1} />
          ))}
        </ol>
      ) : null}
    </li>
  );
}, sameProps);

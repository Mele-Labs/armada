// Doctor, the health surface, drawn as the cockpit is: a sunken glass under a header band, one
// instrument per module with its state in the frame, and a pane for the one picked. A result is a
// word in its status colour and never a glyph (`docs/concepts/doctor.md`). Doctor is a pull: it
// reads on opening and on Check again, and changes nothing.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { CircleCheck, CircleDashed, CircleX, RotateCw, Stethoscope, TriangleAlert } from "lucide-react";
import { Button, Stamp, Tooltip } from "@armada/components";
import type { Connection } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";

import { gridOf, MEANS, readingOf, summaryOf, worstOf, type Row } from "./grid";
import "../cockpit/cockpit.css";
import "./doctor.css";


const GLYPHS = { pass: CircleCheck, warn: TriangleAlert, fail: CircleX, reading: CircleDashed } as const;
const GLYPH_NAMES = { pass: "Pass", warn: "Warn", fail: "Fail", reading: "Reading" } as const;

/** A result as its registry glyph in the result's hue, with the word as its tooltip. Drawn under the new look; the word stays for everything else. */
function Glyph({ result }: { result: Row["result"] }) {
  const Icon = GLYPHS[result];
  return (
    <Tooltip label={GLYPH_NAMES[result]}>
      <span className="armada-doctor__glyph" data-result={result} role="img" aria-label={GLYPH_NAMES[result]}>
        <Icon size={16} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

function Tile({ row, selected, onSelect }: { row: Row; selected: boolean; onSelect: (key: string) => void }) {
  const ref = useRef<HTMLLIElement>(null);
  useEffect(() => {
    if (selected) ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  return (
    <li
      ref={ref}
      role="option"
      className="armada-tile armada-doctor-tile"
      data-result={row.result}
      data-lit={row.result === "fail" || undefined}
      aria-label={`${row.module}, ${row.result}`}
      aria-selected={selected}
      onClick={() => onSelect(row.key)}
    >
      <div className="armada-tile__head">
        <Glyph result={row.result} />
        <span className="armada-doctor-tile__module">{row.module}</span>
        <b className="armada-doctor__result" data-result={row.result} data-chip="">
          {row.result}
        </b>
      </div>
      {MEANS[row.module] === undefined ? null : <span className="armada-doctor-tile__checks">{MEANS[row.module]!.reads}</span>}
      <ReadingOf detail={row.detail} />
      <div className="armada-tile__foot">
        <span className="armada-doctor-tile__owner">{`Probed by ${row.owner}`}</span>
      </div>
    </li>
  );
}

/** A probe's line laid out: its sentences, then each value it named, label beside value. */
function ReadingOf({ detail, whole = false }: { detail: string; whole?: boolean }) {
  const { lines, facts } = readingOf(detail);
  if (lines.length === 0 && facts.length === 0) return <span className="armada-doctor-reading__none">Nothing read yet</span>;
  return (
    <div className="armada-doctor-reading" data-whole={whole || undefined}>
      {lines.map((line, at) => (
        <p key={at} className="armada-doctor-reading__line">
          {line}
        </p>
      ))}
      {facts.length === 0 ? null : (
        <dl className="armada-doctor-reading__facts">
          {facts.map((fact, at) => (
            <div key={at}>
              <dt>{fact.label}</dt>
              <dd data-mono={fact.mono || undefined} title={fact.full}>
                {whole && fact.full !== undefined ? fact.full : fact.value}
              </dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

/** The picked module whole: what it read, who probes it, and what each of its words would mean. */
function Pane({ row }: { row: Row }) {
  const means = MEANS[row.module];
  const words = (["pass", "warn", "fail"] as const).filter((word) => means === undefined || word !== "warn" || means.warn !== undefined);
  return (
    <article className="armada-doctor-pane" data-result={row.result} aria-label={row.module}>
      <header className="armada-doctor-pane__head">
        <Glyph result={row.result} />
        <h2>{row.module}</h2>
        <b className="armada-doctor__result" data-result={row.result}>
          {row.result}
        </b>
      </header>
      <dl className="armada-doctor-pane__facts">
        <dt>Read</dt>
        <dd>
          <ReadingOf detail={row.detail} whole />
        </dd>
        <dt>Probed by</dt>
        <dd>{row.owner}</dd>
        {means === undefined ? null : (
          <>
            <dt>Checks</dt>
            <dd>{means.reads}</dd>
          </>
        )}
      </dl>
      {means === undefined ? null : (
        <ul className="armada-doctor-pane__words" aria-label="What each result means">
          {words.map((word) => (
            <li key={word} data-current={row.result === word || undefined}>
              <b className="armada-doctor__result" data-result={word}>
                {word}
              </b>
              <span>{word === "warn" ? means.warn : means[word]}</span>
            </li>
          ))}
          {means.warn === undefined ? <li className="armada-doctor-pane__never">Never warns: it is reachable or it is not.</li> : null}
        </ul>
      )}
    </article>
  );
}

export function DoctorSurface({
  health,
  connection,
  said,
  now,
  onCheck,
}: {
  health: HealthRead;
  connection: Connection;
  /** The connection's own sentence, as the Fleet panel says it. */
  said: string;
  now: number;
  /** Ask every probe again. */
  onCheck: () => void;
}) {
  const grid = gridOf(health, connection, said);
  const worst = worstOf(grid.rows);
  const reading = health.state === "reading";

  // Checked on demand: opening Doctor is a demand.
  useEffect(() => {
    onCheck();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // When the probes last answered, read off the app's one clock.
  const [checkedAt, setCheckedAt] = useState<string>();
  useEffect(() => {
    if (health.state === "read" || health.state === "failed") setCheckedAt(new Date().toISOString());
  }, [health]);

  const [selected, setSelected] = useState<string>();
  const at = Math.max(0, grid.rows.findIndex((row) => row.key === selected));
  const current = grid.rows[at];
  const list = useRef<HTMLUListElement>(null);
  const move = (event: KeyboardEvent) => {
    const cols = list.current === null ? 1 : Math.max(1, getComputedStyle(list.current).gridTemplateColumns.split(" ").length);
    const by = { ArrowRight: 1, ArrowLeft: -1, j: 1, k: -1, ArrowDown: cols, ArrowUp: -cols }[event.key];
    if (by === undefined || event.metaKey || event.ctrlKey || event.altKey) return;
    event.preventDefault();
    const next = grid.rows[Math.min(grid.rows.length - 1, Math.max(0, at + by))];
    if (next !== undefined) setSelected(next.key);
  };

  return (
    <div className="armada-cockpit armada-doctor" data-worst={worst}>
      <section className="armada-view" aria-label="Doctor" data-waiting={worst === undefined ? undefined : ""}>
        <header className="armada-view__band">
          <Stethoscope size={16} aria-hidden="true" />
          <span className="armada-doctor__title">Doctor</span>
          <span className="armada-doctor__summary" data-worst={worst}>
            {summaryOf(grid)}
          </span>
          <span className="armada-view__keys">
            {checkedAt === undefined ? null : (
              <span>
                checked <Stamp first="age" at={checkedAt} now={now} />
              </span>
            )}
            <Button variant="ghost" size="sm" onClick={onCheck} disabled={reading}>
              <RotateCw size={12} aria-hidden="true" />
              Check again
            </Button>
          </span>
        </header>
        <div className="armada-view__stage">
          <div className="armada-view__body" data-pane={current === undefined ? undefined : ""}>
            <ul ref={list} className="armada-tiles armada-view__grid" role="listbox" aria-label="Modules" tabIndex={0} onKeyDown={move}>
              {grid.rows.map((row) => (
                <Tile key={row.key} row={row} selected={row.key === current?.key} onSelect={setSelected} />
              ))}
            </ul>
            {current === undefined ? null : (
              <div className="armada-view__pane">
                <Pane row={current} />
              </div>
            )}
          </div>
        </div>
        {grid.gaps.length === 0 ? null : (
          <footer className="armada-view__horizon armada-doctor__gaps" aria-label="Not probed">
            <span className="armada-doctor__gaps-head">Not probed yet</span>
            <ul>
              {grid.gaps.map((gap) => (
                <li key={gap.owner}>
                  <span className="armada-doctor__gap-owner">{gap.owner}</span>
                  <span className="armada-doctor__gap-because">{gap.because}</span>
                </li>
              ))}
            </ul>
          </footer>
        )}
      </section>
    </div>
  );
}

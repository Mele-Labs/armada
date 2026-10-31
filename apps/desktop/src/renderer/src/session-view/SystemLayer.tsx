// The System layer's sections for one Job: its processes, CPU, memory and disk, the worktrees it holds and its logs. Each section draws
// where the Job has something for it; the processes keep fixed columns where a row lacks a port.

import type { ReactNode } from "react";
import { CircleDot, Power, Square } from "lucide-react";
import { Button, DropdownMenu, Skeleton, Tooltip, useKept } from "@armada/components";
import { SlidersHorizontal } from "lucide-react";

import { openInPanel } from "../shared-panel/store";
import { HISTORY, sizeOf, stopProcess, useSystem, type LogFile, type Process, type Reading } from "./system";

type Column = "port" | "cpu" | "memory" | "io";
const COLUMNS: readonly { id: Column; label: string }[] = [
  { id: "port", label: "Port" },
  { id: "cpu", label: "CPU" },
  { id: "memory", label: "Memory" },
  { id: "io", label: "I/O" },
];
const COLUMNS_KEY = "system-process-columns";
const MB = 1024 ** 2;

const cell = (one: Process, id: Column): string => {
  if (id === "port") return one.port === undefined ? "" : `:${one.port}`;
  if (id === "cpu") return one.cpu === undefined ? "" : `${Math.round(one.cpu)}%`;
  if (id === "memory") return one.memory === undefined ? "" : one.memory >= 1024 * MB ? `${(one.memory / 1024 / MB).toFixed(1)} GB` : `${Math.round(one.memory / MB)} MB`;
  return one.io === undefined ? "" : `${sizeOf(one.io)}/s`;
};

const STATE = { running: { Glyph: CircleDot, said: "Running" }, stopped: { Glyph: Power, said: "Stopped" } } as const;

const template = (columns: readonly Column[]): string => `minmax(20ch, 1fr) ${columns.map(() => "7ch").join(" ")} 7ch var(--space-6) var(--space-8)`;

function ProcessHeader({ columns }: { columns: readonly Column[] }) {
  return (
    <li className="sv-process sv-process--head" style={{ gridTemplateColumns: template(columns) }}>
      <span>Command</span>
      {columns.map((id) => (
        <span key={id} className="sv-process__cell">
          {COLUMNS.find((c) => c.id === id)?.label}
        </span>
      ))}
      <span className="sv-process__cell">PID</span>
      <span />
      <span />
    </li>
  );
}

function ProcessRow({ sessionId, one, columns }: { sessionId: string; one: Process; columns: readonly Column[] }) {
  const { Glyph, said } = STATE[one.state];
  return (
    <li className="sv-process" data-state={one.state} aria-label={one.command} style={{ gridTemplateColumns: template(columns) }}>
      <code className="sv-process__command">{one.command}</code>
      {columns.map((id) => (
        <span key={id} className="sv-process__cell">
          {cell(one, id)}
        </span>
      ))}
      <span className="sv-process__pid sv-process__cell">{one.pid}</span>
      <Tooltip label={said}>
        <span className="sv-layer__mark sv-process__state" data-tone={one.state} role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-process__act">
        {one.state === "running" ? (
          <Tooltip asChild label={`Stop ${one.command}`}>
            <Button variant="secondary" size="sm" iconOnly aria-label={`Stop ${one.command}`} onClick={() => stopProcess(sessionId, one.pid)}>
              <Square size={10} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        ) : null}
      </span>
    </li>
  );
}

const figure = (one: Reading): string => (one.unit === "%" ? `${one.used}%` : one.unit === "GB" ? `${one.used.toFixed(1)} of ${one.of} GB` : `${one.used} of ${one.of} ${one.unit}`);

/** The last readings as a filled line, newest at the right edge; `of` is the top of the box. */
function Graph({ name, one, past }: { name: string; one: Reading; past: readonly number[] }) {
  const W = 120;
  const H = 32;
  const at = (n: number, i: number) => `${((i / (HISTORY - 1)) * W).toFixed(1)},${(H - Math.min(1, n / one.of) * H).toFixed(1)}`;
  const first = HISTORY - past.length;
  const line = past.map((n, i) => at(n, first + i)).join(" ");
  return (
    <svg className="sv-graph" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" role="img" aria-label={`${name}, last ${past.length} readings`}>
      {past.length < 2 ? null : (
        <>
          <polygon className="sv-graph__area" points={`${at(0, first).split(",")[0]},${H} ${line} ${W},${H}`} />
          <polyline className="sv-graph__line" points={line} vectorEffect="non-scaling-stroke" />
        </>
      )}
    </svg>
  );
}

function ReadingRow({ name, one, past }: { name: string; one: Reading; past?: readonly number[] }) {
  return (
    <li className="sv-reading">
      {past !== undefined ? (
        <Graph name={name} one={one} past={past} />
      ) : (
        <span className="armada-usage__meter sv-reading__bar" role="meter" aria-label={name} aria-valuemin={0} aria-valuemax={one.of} aria-valuenow={one.used} aria-valuetext={figure(one)}>
          <span className="armada-usage__fill" style={{ width: `${Math.min(100, (one.used / one.of) * 100)}%` }} />
        </span>
      )}
    </li>
  );
}

function LogRow({ sessionId, one }: { sessionId: string; one: LogFile }) {
  return (
    <li>
      <button type="button" className="sv-log" aria-label={`Open ${one.name}`} onClick={() => openInPanel({ kind: "file", path: one.path, from: "session", sessionId })}>
        <code className="sv-log__name">{one.name}</code>
        <span className="sv-log__size">{sizeOf(one.bytes)}</span>
      </button>
    </li>
  );
}

function Section({ label, note, aside, children }: { label: string; note?: string; aside?: ReactNode; children: ReactNode }) {
  return (
    <section className="sv-layer__group" aria-label={label}>
      <h4 className="sv-layer__group-label">
        {label}
        {note === undefined ? null : <span className="sv-layer__group-note"> · {note}</span>}
        {aside === undefined ? null : <span className="sv-layer__group-aside">{aside}</span>}
      </h4>
      {children}
    </section>
  );
}

function ColumnMenu({ shown, onChange }: { shown: readonly Column[]; onChange: (next: Column[]) => void }) {
  return (
    <DropdownMenu
      triggerLabel="Columns"
      icon={SlidersHorizontal}
      entries={COLUMNS.map((c) => ({ kind: "item" as const, id: c.id, label: c.label, selected: shown.includes(c.id) }))}
      onSelect={(id) => onChange(shown.includes(id as Column) ? shown.filter((c) => c !== id) : [...shown, id as Column])}
    />
  );
}

/** Placeholder rows for each section, drawn until the first reading arrives. */
function Loading() {
  return (
    <div aria-busy="true" aria-label="Reading the system">
      {[["Processes", 5], ["CPU", 1], ["Memory", 1], ["Disk", 1]].map(([label, rows]) => (
        <Section key={label} label={String(label)}>
          <ul className="sv-layer__rows">
            {Array.from({ length: Number(rows) }, (_, i) => (
              <li key={i} className="sv-skeleton">
                <Skeleton width={`${90 - i * 7}%`} />
              </li>
            ))}
          </ul>
        </Section>
      ))}
    </div>
  );
}

/** `worktrees` is what the Slot layer already draws for each bay the Job holds, or nothing. */
export function SystemSections({ sessionId, worktrees }: { sessionId: string; worktrees: ReactNode }) {
  const { processes, cpu, memory, disk, logs, readAt, history } = useSystem(sessionId);
  const [shown, setShown] = useKept<Column[]>(COLUMNS_KEY, ["port"]);
  if (readAt === undefined) return <Loading />;
  return (
    <>
        <Section label="Processes" aside={<ColumnMenu shown={shown} onChange={setShown} />} note={`read ${new Date(readAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" })}`}>
          <ul className="sv-layer__rows sv-layer__rows--fixed">
            <ProcessHeader columns={COLUMNS.map((c) => c.id).filter((id) => shown.includes(id))} />
            {processes.map((one) => (
              <ProcessRow key={one.pid} sessionId={sessionId} one={one} columns={COLUMNS.map((c) => c.id).filter((id) => shown.includes(id))} />
            ))}
          </ul>
        </Section>
      {cpu === undefined ? null : (
        <Section label="CPU" aside={figure(cpu)}>
          <ul className="sv-layer__rows">
            <ReadingRow name="CPU" one={cpu} past={history.cpu} />
          </ul>
        </Section>
      )}
      {memory === undefined ? null : (
        <Section label="Memory" aside={figure(memory)}>
          <ul className="sv-layer__rows">
            <ReadingRow name="Memory" one={memory} past={history.memory} />
          </ul>
        </Section>
      )}
      {disk === undefined ? null : (
        <Section label="Disk" aside={figure(disk)}>
          <ul className="sv-layer__rows">
            <ReadingRow name="Disk" one={disk} />
          </ul>
        </Section>
      )}
      {worktrees === null ? null : <Section label="Worktrees">{worktrees}</Section>}
      {logs.length === 0 ? null : (
        <Section label="Logs">
          <ul className="sv-layer__rows">
            {logs.map((one) => (
              <LogRow key={one.path} sessionId={sessionId} one={one} />
            ))}
          </ul>
        </Section>
      )}
    </>
  );
}

// The System layer's sections for one Job: its processes, CPU, memory and disk, the worktrees it holds and its logs. Each section draws
// where the Job has something for it; the processes keep fixed columns where a row lacks a port.

import type { ReactNode } from "react";
import { CircleDot, Power } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { openInPanel } from "../shared-panel/store";
import { sizeOf, stopProcess, useSystem, type LogFile, type Process, type Reading } from "./system";

const STATE = { running: { Glyph: CircleDot, said: "Running" }, stopped: { Glyph: Power, said: "Stopped" } } as const;

function ProcessRow({ sessionId, one }: { sessionId: string; one: Process }) {
  const { Glyph, said } = STATE[one.state];
  return (
    <li className="sv-process" data-state={one.state} aria-label={one.command}>
      <code className="sv-process__command">{one.command}</code>
      <span className="sv-process__port">{one.port === undefined ? null : `:${one.port}`}</span>
      <span className="sv-process__pid">{one.pid}</span>
      <Tooltip label={said}>
        <span className="sv-layer__mark sv-process__state" data-tone={one.state} role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-process__act">
        {one.state === "running" ? (
          <Tooltip asChild label={`Stop ${one.command}`}>
            <Button variant="ghost" size="sm" iconOnly aria-label={`Stop ${one.command}`} onClick={() => stopProcess(sessionId, one.pid)}>
              <Power size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        ) : null}
      </span>
    </li>
  );
}

const figure = (one: Reading): string => (one.unit === "GB" ? `${one.used.toFixed(1)} of ${one.of} GB` : `${one.used} of ${one.of} ${one.unit}`);

function ReadingRow({ name, one }: { name: string; one: Reading }) {
  return (
    <li className="sv-reading">
      <span className="sv-reading__value">{figure(one)}</span>
      <span className="armada-usage__meter sv-reading__bar" role="meter" aria-label={name} aria-valuemin={0} aria-valuemax={one.of} aria-valuenow={one.used} aria-valuetext={figure(one)}>
        <span className="armada-usage__fill" style={{ width: `${Math.min(100, (one.used / one.of) * 100)}%` }} />
      </span>
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

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="sv-layer__group" aria-label={label}>
      <h4 className="sv-layer__group-label">{label}</h4>
      {children}
    </section>
  );
}

/** `worktrees` is what the Slot layer already draws for each bay the Job holds, or nothing. */
export function SystemSections({ sessionId, worktrees }: { sessionId: string; worktrees: ReactNode }) {
  const { processes, cpu, memory, disk, logs } = useSystem(sessionId);
  return (
    <>
      {processes.length === 0 ? null : (
        <Section label="Processes">
          <ul className="sv-layer__rows">
            {processes.map((one) => (
              <ProcessRow key={one.pid} sessionId={sessionId} one={one} />
            ))}
          </ul>
        </Section>
      )}
      {cpu === undefined ? null : (
        <Section label="CPU">
          <ul className="sv-layer__rows">
            <ReadingRow name="CPU" one={cpu} />
          </ul>
        </Section>
      )}
      {memory === undefined ? null : (
        <Section label="Memory">
          <ul className="sv-layer__rows">
            <ReadingRow name="Memory" one={memory} />
          </ul>
        </Section>
      )}
      {disk === undefined ? null : (
        <Section label="Disk">
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

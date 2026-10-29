// The Processes section of the Artifacts layer: one row per process the Job started, in fixed columns kept where a row lacks a port.

import { CircleDot, Power } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { stopProcess, useProcesses, type Process } from "./processes";

const STATE = { running: { Glyph: CircleDot, said: "Running" }, stopped: { Glyph: Power, said: "Stopped" } } as const;

function ProcessRow({ sessionId, one }: { sessionId: string; one: Process }) {
  const { Glyph, said } = STATE[one.state];
  return (
    <li className="sv-process" data-state={one.state}>
      <code className="sv-process__command">{one.command}</code>
      <span className="sv-process__port">{one.port === undefined ? null : `:${one.port}`}</span>
      <span className="sv-process__pid">{one.pid}</span>
      <Tooltip label={said}>
        <span className="sv-layer__mark sv-process__state" data-tone={one.state === "running" ? "running" : "stopped"} role="img" aria-label={said}>
          <Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-process__act">
        {one.state === "running" ? (
          <Button variant="ghost" size="sm" aria-label={`Stop ${one.command}`} onClick={() => stopProcess(sessionId, one.pid)}>
            Stop
          </Button>
        ) : null}
      </span>
    </li>
  );
}

/** Nothing is drawn for a Job that started none, or for words that match none. */
export function ProcessesGroup({ sessionId, words }: { sessionId: string; words: string }) {
  const query = words.trim().toLowerCase();
  const rows = useProcesses(sessionId).filter((one) => query === "" || "processes".includes(query) || `${one.command} ${one.port ?? ""} ${one.pid}`.toLowerCase().includes(query));
  if (rows.length === 0) return null;
  return (
    <section className="sv-layer__group" aria-label="Processes">
      <h4 className="sv-layer__group-label">Processes</h4>
      <ul className="sv-layer__rows">
        {rows.map((one) => (
          <ProcessRow key={one.pid} sessionId={sessionId} one={one} />
        ))}
      </ul>
    </section>
  );
}

import { createContext, useContext } from "react";
import { Check, ChevronRight, Unplug, X } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionThreadRow } from "./SessionThread";

export type RequestedThreadRow = Extract<SessionThreadRow, { kind: "requested" }>;

/**
 * Opens a request the thread holds in Requests, to edit and send again. **A context and not a prop of the thread**, so the
 * host that serves Requests hands it to the rows it draws. Absent, a request's row has no press for it.
 */
export const RequestOpen = createContext<((row: RequestedThreadRow) => void) | null>(null);

/** A request's time as it is read: milliseconds under a second, seconds to one place past it. */
export const requestTimeOf = (ms: number): string => (ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`);

/**
 * How a request was answered: a check for a 2xx or 3xx, a cross for a 4xx or 5xx, and the unplugged mark where nothing
 * answered. The tooltip is the status, or Fleet's words for why nothing did.
 */
export function RequestMark({ status, error }: { status: number | null; error?: string | undefined }) {
  const outcome = status === null ? "none" : status < 400 ? "answered" : "failed";
  const said = status === null ? (error === undefined ? "No answer" : `No answer: ${error}`) : String(status);
  const Glyph = outcome === "none" ? Unplug : outcome === "answered" ? Check : X;
  return (
    <Tooltip label={said}>
      <span className="armada-request-mark" data-outcome={outcome} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * An HTTP request sent through Fleet, by the person or by the agent, in the thread where it was sent: who, how it was
 * answered, the method, the address and the time on one line, and the answer's body as plain text under the fold. The
 * request is on the row whole, so `RequestOpen` puts it back in Requests to edit and send again.
 */
export function RequestedRow({ row, agentName }: { row: RequestedThreadRow; agentName: string }) {
  const onOpen = useContext(RequestOpen);
  const by = row.by === "owner" ? "You" : agentName;
  const answered = row.status === null ? "No answer" : String(row.status);
  return (
    <li className="armada-session-fold armada-session-fold--requested">
      <details>
        <summary className="armada-session-fold__head" aria-label={`${by}: ${answered} ${row.method} ${row.url}`}>
          <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-session-fold__chevron" />
          <RequestMark status={row.status} error={row.error} />
          <span className="armada-requested__by">{by}</span>
          <code className="armada-requested__method">{row.method}</code>
          <code className="armada-session-fold__target">{row.url}</code>
          <span className="armada-requested__ms">{requestTimeOf(row.ms)}</span>
        </summary>
        <div className="armada-requested__body">
          {row.error === undefined ? null : <p className="armada-requested__error">{row.error}</p>}
          {row.response === "" ? null : (
            <pre className="armada-session-fold__tail" role="log" aria-label="Response">
              {row.response}
            </pre>
          )}
          {row.truncated ? <p className="armada-requested__cut">Cut at 16 KiB</p> : null}
          {onOpen === null ? null : (
            <span className="armada-requested__acts">
              <Button variant="ghost" size="sm" onClick={() => onOpen(row)}>
                Open in Requests
              </Button>
            </span>
          )}
        </div>
      </details>
    </li>
  );
}

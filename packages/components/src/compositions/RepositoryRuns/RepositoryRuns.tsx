import type { ReactNode } from "react";
import { Check, CircleDashed, Terminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * How a repository runs, by job: the command Fleet saw pass for it, and the port it listened on. A state is a glyph with a tooltip.
 * **Mock only**, drawn in the dispatch panel and in the repository picker's own place.
 */
export type RunsRow = {
  id: string;
  job: string;
  command?: string;
  port?: number;
  state: "seen" | "not_seen" | "remembered";
};

const STATE: Record<RunsRow["state"], { Glyph: LucideIcon; word: string }> = {
  seen: { Glyph: Terminal, word: "Seen in a Session" },
  not_seen: { Glyph: CircleDashed, word: "Not seen" },
  remembered: { Glyph: Check, word: "Remembered" },
};

export function RepositoryRuns({ name, source, rows, acts }: { name: string; source: string; rows: readonly RunsRow[]; acts?: ReactNode }) {
  return (
    <section className="armada-runs" role="group" aria-label={`How ${name} runs`}>
      <header className="armada-runs__head">
        <span className="armada-runs__title">How {name} runs</span>
        <span className="armada-runs__source">{source}</span>
      </header>
      <ul className="armada-runs__rows">
        {rows.map((one) => {
          const { Glyph, word } = STATE[one.state];
          return (
            <li key={one.id} className="armada-runs__row" data-state={one.state}>
              <span className="armada-runs__job">{one.job}</span>
              <span className="armada-runs__how">
                <code className="armada-runs__command">{one.command ?? ""}</code>
                {one.port === undefined ? null : <code className="armada-runs__port">{`:${one.port}`}</code>}
              </span>
              <Tooltip label={word}>
                <span className="armada-runs__state" role="img" aria-label={`${one.job}: ${word}`}>
                  <Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            </li>
          );
        })}
      </ul>
      {acts === undefined ? null : <div className="armada-runs__acts">{acts}</div>}
    </section>
  );
}

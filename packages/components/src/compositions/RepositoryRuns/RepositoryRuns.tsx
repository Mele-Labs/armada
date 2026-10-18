import type { ReactNode } from "react";
import { Check, Terminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { PLACE_GLYPH, placePhrase } from "./RunsPlaces";
import type { RunsPlace } from "./RunsPlaces";

export * from "./RunsPlaces";

/**
 * How a repository runs, by job: the command Fleet saw pass for it, and the port it listened on. A state is a glyph with a tooltip, and a
 * remembered one wears the glyph of the place it is kept in. **Mock only**, drawn in the dispatch panel.
 */
export type RunsRow = {
  id: string;
  job: string;
  command?: string;
  port?: number;
  state: "seen" | "remembered";
};

const STATE: Record<RunsRow["state"], { Glyph: LucideIcon; word: string }> = {
  seen: { Glyph: Terminal, word: "Seen in a Job" },
  remembered: { Glyph: Check, word: "Remembered" },
};

export function RepositoryRuns({
  name,
  source,
  rows,
  place,
  onChange,
  changing,
  chooser,
  acts,
}: {
  name: string;
  source: string;
  rows: readonly RunsRow[];
  /** Where the runs are kept. Absent until the first Remember. */
  place?: RunsPlace | undefined;
  /** Opens where they are kept, to change it. Drawn once there is a place. */
  onChange?: (() => void) | undefined;
  changing?: boolean | undefined;
  /** The places to choose from, in place under the rows. */
  chooser?: ReactNode;
  acts?: ReactNode;
}) {
  const kept = place === undefined ? undefined : { Glyph: PLACE_GLYPH[place.id], word: `Remembered ${placePhrase(place)}` };
  return (
    <section className="armada-runs" role="group" aria-label={`How ${name} runs`}>
      <header className="armada-runs__head">
        <span className="armada-runs__title">How {name} runs</span>
        <span className="armada-runs__source">{source}</span>
        {kept === undefined || place === undefined || onChange === undefined ? null : (
          <Tooltip label={`Kept ${placePhrase(place)}`}>
            <button type="button" className="armada-runs__keep" aria-label={`Kept ${placePhrase(place)}`} aria-expanded={changing === true} onClick={onChange}>
              <kept.Glyph size={12} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
        )}
      </header>
      <ul className="armada-runs__rows">
        {rows.map((one) => {
          const { Glyph, word } = one.state === "remembered" && kept !== undefined ? kept : STATE[one.state];
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
      {chooser}
      {acts === undefined ? null : <div className="armada-runs__acts">{acts}</div>}
    </section>
  );
}

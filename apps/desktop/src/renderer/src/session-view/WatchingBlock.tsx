// Watching: the watches a Job holds, in a block of their own in the Landing, below its pull requests (the owner chose it over nesting each under its
// pull request, since a watch can follow none). A row is the state's mark, the watch's name, what its last reading said and how long ago; under
// it, what the agent made of that reading where it weighed it. A press opens the watch in the side panel.

import { Stamp } from "@armada/components";
import type { WatchSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { openInPanel } from "../shared-panel/store";
import { readingSays, verdictSaid, verdictsOn, WatchMark } from "./watch-marks";
import "./watching-block.css";

export const openWatchPanel = (sessionId: string, watch: WatchSummary) => openInPanel({ kind: "watch", sessionId, id: watch.id, name: watch.name });

function WatchRow({ sessionId, watch }: { sessionId: string; watch: WatchSummary }) {
  const last = watch.last;
  const weighed = last === undefined ? [] : verdictsOn(watch, last);
  return (
    <li className="sv-watch" data-state={watch.state}>
      <button type="button" className="sv-watch__row" aria-label={`Watch ${watch.name}`} onClick={() => openWatchPanel(sessionId, watch)}>
        <WatchMark state={watch.state} />
        <span className="sv-watch__name">{watch.name}</span>
        <span className="sv-watch__says">{last === undefined ? "" : readingSays(last)}</span>
        {last === undefined ? <span /> : <Stamp at={last.at} first="age" still className="sv-watch__age" />}
      </button>
      {weighed.length === 0 ? null : (
        <ul className="sv-watch__verdicts" aria-label={`Weighed by the agent: ${watch.name}`}>
          {weighed.map((one) => (
            <li key={`${one.at}:${one.verdict}`} className="sv-watch__verdict" data-verdict={one.verdict}>
              {verdictSaid(one)}
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

export function WatchingBlock({ session }: { session: Session }) {
  const watches = session.watches ?? [];
  if (watches.length === 0) return null;
  return (
    <section className="sv-watching" aria-label="Watching">
      <span className="sv-watching__label">Watching</span>
      <ul className="sv-watching__list">
        {watches.map((watch) => (
          <WatchRow key={watch.id} sessionId={session.id} watch={watch} />
        ))}
      </ul>
    </section>
  );
}

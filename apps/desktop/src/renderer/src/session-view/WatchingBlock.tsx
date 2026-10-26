// Watching: the watches a Job holds, in a block of their own in the Landing, below its pull requests (the owner chose it over nesting each under its
// pull request, since a watch can follow none). A row is the state's mark, the watch's name, what its last reading said and how long ago; under
// it, what the agent made of that reading where it weighed it. A press opens the watch in the side panel.
//
// Live watches only, and one that ended not ours while Needs you still asks about it, until Show ended draws the ended ones too, each with
// Delete in a column every row keeps.

import { useState } from "react";
import { Trash2 } from "lucide-react";
import { Button, Stamp, Switch, Tooltip } from "@armada/components";
import type { WatchSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../sessions-draft";
import { openInPanel } from "../shared-panel/store";
import { readingSays, verdictSaid, verdictsOn, watchesShown, watchSaid, WatchMark } from "./watch-marks";
import "./watching-block.css";

export const openWatchPanel = (sessionId: string, watch: WatchSummary) => openInPanel({ kind: "watch", sessionId, id: watch.id, name: watch.name });

function WatchRow({ sessionId, watch, onDelete }: { sessionId: string; watch: WatchSummary; onDelete: ((watch: string) => void) | undefined }) {
  const last = watch.last;
  const weighed = last === undefined ? [] : verdictsOn(watch, last);
  return (
    <li className="sv-watch" data-state={watch.state}>
      <div className="sv-watch__line">
        <button type="button" className="sv-watch__row" aria-label={`Watch ${watch.name}`} onClick={() => openWatchPanel(sessionId, watch)}>
          <WatchMark state={watch.state} said={watchSaid(watch)} />
          <span className="sv-watch__name">{watch.name}</span>
          <span className="sv-watch__says">{last === undefined ? "" : readingSays(last)}</span>
          {last === undefined ? <span /> : <Stamp at={last.at} first="age" still className="sv-watch__age" />}
        </button>
        <span className="sv-watch__act">
          {watch.live || onDelete === undefined ? null : (
            <Tooltip label="Delete">
              <Button variant="ghost" size="sm" iconOnly aria-label={`Delete watch ${watch.name}`} onClick={() => onDelete(watch.id)}>
                <Trash2 size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
        </span>
      </div>
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
  const draft = useSessionsDraft();
  const [ended, setEnded] = useState(false);
  const watches = session.watches ?? [];
  if (watches.length === 0) return null;
  const anyEnded = watches.some((watch) => !watch.live);
  const shown = watchesShown(watches, session.waitingFor ?? [], ended);
  const remove = draft?.deleteWatch;
  return (
    <section className="sv-watching" aria-label="Watching">
      <div className="sv-watching__head">
        <span className="sv-watching__label">Watching</span>
        {anyEnded ? (
          <Switch size="sm" checked={ended} onChange={(event) => setEnded(event.currentTarget.checked)}>
            Show ended
          </Switch>
        ) : null}
      </div>
      {shown.length === 0 ? null : (
        <ul className="sv-watching__list" data-acts={anyEnded || undefined}>
          {shown.map((watch) => (
            <WatchRow key={watch.id} sessionId={session.id} watch={watch} onDelete={remove === undefined ? undefined : (id) => remove(session.id, id)} />
          ))}
        </ul>
      )}
    </section>
  );
}

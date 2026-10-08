// Command Central: the queue of what needs the owner, one compact row each, and the one selected
// opened beside it with the context to answer it. With nothing waiting, the fleet board is drawn
// alone. Mock only, as the Dashboard is.

import { useState, type KeyboardEvent } from "react";
import { Tooltip } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";

import type { BridgeState } from "../../shared/bridge";
import { CallPane } from "./CallPane";
import { age, onListKey, useBoardKeys, useCursor, useItems, type Hosts } from "./Dashboard";
import { FleetBoard } from "./FleetBoard";

export function CommandCentral({
  state,
  now,
  picked,
  nowViews,
  ...hosts
}: Hosts & {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
}) {
  const [answered, setAnswered] = useState<ReadonlySet<string>>(new Set());
  const calls = useItems("command-central", state, picked, nowViews, hosts, answered);
  const [selected, setSelected] = useState<string>();
  const at = Math.max(0, calls.findIndex((one) => one.key === selected));
  const call = calls[at];
  useCursor(call?.job, hosts.onCursor);
  useBoardKeys(calls, at, setSelected, hosts, call !== undefined);

  if (call === undefined) return <FleetBoard state={state} now={now} picked={picked} nowViews={nowViews} pane={false} {...hosts} />;

  const move = (event: KeyboardEvent) => onListKey(event, calls, at, setSelected);
  // The next call steps up: the one that was below it, or the one above where it was last.
  const done = () => {
    setSelected((calls[at + 1] ?? calls[at - 1])?.key);
    setAnswered(new Set([...answered, call.key]));
  };

  return (
    <div className="armada-deck">
      <ul className="armada-queue" role="listbox" aria-label="Needs you" tabIndex={0} onKeyDown={move}>
        {calls.map((one) => {
          const Icon = one.icon;
          return (
            <li key={one.key} role="option" aria-selected={one.key === call.key} data-hue={one.hue} data-job-id={one.job?.id} data-status={one.job?.status} className="armada-queue__row" onClick={() => setSelected(one.key)}>
              <Tooltip label={one.kind}>
                <span className="armada-queue__mark">
                  <Icon size={16} aria-hidden="true" />
                </span>
              </Tooltip>
              <span className="armada-queue__title">{one.title}</span>
              <span className="armada-queue__age">{age(one.at, now)}</span>
              <span className="armada-queue__fact">{one.fact}</span>
            </li>
          );
        })}
      </ul>
      <CallPane item={call} now={now} workflows={state.holds.workflows} onDone={done} onOpenSession={hosts.onOpenSession} onOpenJob={hosts.onOpen} onOpenLink={hosts.onOpenLink} />
    </div>
  );
}

// The fleet board: every live Job and Session a tile in one grid, and the merge line as a conveyor
// beneath. On Running the grid takes the left column and the tile picked opens its context pane beside
// it; on an idle Command Central the grid is drawn alone and a press opens the Job or Session. Mock
// only, as the Dashboard is.

import { useContext, useMemo, useState, type KeyboardEvent } from "react";
import { CircleDashed, GitMerge } from "lucide-react";
import { Tooltip } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import { nowPanelOf } from "@armada/jobs";
import { overviewListsOf } from "@armada/overview";

import type { BridgeState } from "../../shared/bridge";
import { CallPane } from "./CallPane";
import { titleOf } from "@armada/screens";
import { FleetTile, onTilesKey } from "./FleetTile";
import { Nows, useBoardKeys, useCursor, useItems, type Hosts, type Item } from "./Dashboard";
import { viewsOf } from "./merge-line";

/** The merge line as a conveyor: each branch a block moving toward main, main's light at the end. */
function Conveyor({ state }: { state: BridgeState }) {
  const views = viewsOf(state);
  if (views.length === 0) return null;
  return (
    <>
      {views.map((view) => {
        const main = view.hub?.main;
        const red = main?.state === "red";
        return (
          <div key={view.root} className="armada-conveyor" aria-label="Merge line">
            <GitMerge size={14} aria-hidden="true" />
            <ol className="armada-conveyor__belt">
              {view.line.map((entry) => (
                <Tooltip key={entry.branch} label={`${entry.branch}, ${entry.state}`}>
                  <li className="armada-conveyor__block" data-state={entry.state} />
                </Tooltip>
              ))}
            </ol>
            <Tooltip label={red ? "Main is red" : "Main is green"}>
              <span className="armada-conveyor__main" data-red={red || undefined}>main</span>
            </Tooltip>
          </div>
        );
      })}
    </>
  );
}

export function FleetBoard({
  state,
  now,
  picked,
  nowViews,
  pane,
  ...hosts
}: Hosts & {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
  /** Whether a tile opens its context beside the board. */
  pane: boolean;
}) {
  const [answered, setAnswered] = useState<ReadonlySet<string>>(new Set());
  const calls = useItems("command-central", state, picked, nowViews, hosts, answered);
  const running = useItems("running", state, picked, nowViews, hosts, answered);
  const nows = useContext(Nows);
  const [selected, setSelected] = useState<string>();

  // What needs the owner leads, then what runs. A call stands in for the Job or Session it is about.
  const tiles = useMemo(() => {
    const read = overviewListsOf(state.jobs, picked);
    // A status this build's registry does not know has no badge to draw: its tile names the status instead.
    const unknown = read.undrawable.map(
      (job): Item => ({ key: job.id, owner: job.id, job, icon: CircleDashed, kind: job.status, title: titleOf(job), fact: job.status, hue: "queued", where: job.handle, body: [["Status", job.status]], acts: () => null }),
    );
    const owned = new Set(calls.flatMap((one) => (one.owner === undefined ? [] : [one.owner])));
    const live = running.filter((one) => !one.key.startsWith("line:") && !owned.has(one.owner ?? one.key));
    return [...calls, ...live, ...unknown];
  }, [state, picked, calls, running]);

  const current = tiles.find((one) => one.key === selected) ?? tiles[0];
  useCursor(pane ? current?.job : undefined, hosts.onCursor);
  const at = tiles.findIndex((one) => one.key === current?.key);
  useBoardKeys(tiles, at, setSelected, hosts, pane);
  const move = (event: KeyboardEvent) => onTilesKey(event, tiles, at, setSelected);
  const open = (key: string) => {
    const one = tiles.find((tile) => tile.key === key);
    if (one?.job !== undefined) hosts.onOpen(one.job.id);
    else if (key.startsWith("session:")) hosts.onOpenSession(key.slice("session:".length));
  };
  const panel = current?.job === undefined || current.decisions !== undefined ? undefined : nowPanelOf(nows?.[current.job.id], { onOpenJob: hosts.onOpen, onSaid: () => {} });

  return (
    <div className="armada-board" data-pane={pane || undefined}>
      <section className="armada-deck__fleet" aria-label="Fleet">
        <ul className="armada-tiles" role={pane ? "listbox" : undefined} aria-label={pane ? "Tiles" : undefined} tabIndex={pane ? 0 : undefined} onKeyDown={pane ? move : undefined}>
          {tiles.map((tile) => (
            <FleetTile key={tile.key} item={tile} selected={pane && tile.key === current?.key} onSelect={pane ? setSelected : open} now={now} />
          ))}
        </ul>
        <Conveyor state={state} />
      </section>
      {pane && current !== undefined ? (
        <CallPane item={current} now={now} workflows={state.holds.workflows} {...(panel === undefined ? {} : { nowPanel: panel })} onDone={() => setAnswered(new Set([...answered, current.key]))} onOpenSession={hosts.onOpenSession} onOpenJob={hosts.onOpen} onOpenLink={hosts.onOpenLink} />
      ) : null}
    </div>
  );
}

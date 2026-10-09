// The Dashboard's panel as a cockpit: a top bar with three filters (Your move, Active, Done) and the
// way to read them, a grid of tiles or a map of stars, and a pane for the one picked. A call that
// needs the owner comes forward over the panel whatever the filter, centred and answerable from the
// keyboard, the calls behind it stacked like a deck; `l` puts the one in front at the back. Mock only.

import { useContext, useEffect, useMemo, useRef, useState } from "react";
import { CircleDashed, GitMerge, LayoutDashboard, Waypoints } from "lucide-react";
import { Button, Kbd, Tabs, Tooltip, actionOf, keyFor } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { nowPanelOf } from "@armada/jobs";
import { isTerminal, titleOf } from "@armada/screens";
import { DASHBOARD_TABS, overviewListsOf, type DashboardTab } from "@armada/overview";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { BridgeState } from "../../../shared/bridge";
import { CallPane } from "../CallPane";
import { Nows, useBoardKeys, useCursor, useItems, type Hosts, type Item } from "../Dashboard";
import { FleetTile } from "../FleetTile";
import { viewsOf } from "../merge-line";
import { useSessions } from "../sessions-draft";
import { CallCard, type CardKeys } from "./CallCard";
import { FleetMap } from "./FleetMap";
import { TAB_KEYS } from "./keys";
import { nearest, skyOf } from "./map-layout";
import { useCockpitView } from "./view";
import "./cockpit.css";

/** Whether motion is off: nothing waits for an exit that will not play. */
const stillness = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The merge line as the glass's horizon: blocks moving toward main's light. */
function Horizon({ state }: { state: BridgeState }) {
  const views = viewsOf(state);
  if (views.length === 0) return null;
  return (
    <>
      {views.map((view) => (
        <div key={view.root} className="armada-view__line" aria-label="Merge line">
          <GitMerge size={16} aria-hidden="true" />
          <ol className="armada-view__belt">
            {view.line.map((entry) => (
              <Tooltip key={entry.branch} label={`${entry.branch}, ${entry.state}`} asChild>
                <li className="armada-view__block" data-state={entry.state} />
              </Tooltip>
            ))}
          </ol>
          <Tooltip label={view.hub?.main?.state === "red" ? "Main is red" : "Main is green"}>
            <span className="armada-view__main" data-red={view.hub?.main?.state === "red" || undefined} />
          </Tooltip>
        </div>
      ))}
    </>
  );
}

/**
 * The calls behind the one in front, stacked like a deck: each a strip a little higher, narrower and
 * dimmer than the one before, its title readable. A call put off is dashed and at the back. Pressing
 * one brings it forward.
 */
function Behind({ edge, put, recall }: { edge: readonly Item[]; put: readonly string[]; recall: (key: string) => void }) {
  return (
    <>
      {edge.slice(0, 3).map((one, index) => {
        const Icon = one.icon;
        return (
          <button
            key={one.key}
            type="button"
            className="armada-behind"
            data-hue={one.hue}
            data-deferred={put.includes(one.key) || undefined}
            style={{ ["--i" as string]: index + 1 }}
            aria-label={`${one.kind}: ${one.title}`}
            onClick={() => recall(one.key)}
          >
            <Icon size={12} aria-hidden="true" />
            <span>{one.title}</span>
            {index === 0 ? <Kbd>w</Kbd> : null}
          </button>
        );
      })}
    </>
  );
}

/** The acts of the tile under the cursor, each with the key that does it. */
function TileActs({ item, hosts }: { item: Item; hosts: Hosts }) {
  const open = () => (item.job === undefined ? hosts.onOpenSession(item.key.slice("session:".length)) : hosts.onOpen(item.job.id));
  const stoppable = item.job !== undefined && !isTerminal(item.job) && hosts.onKill !== undefined;
  return (
    <>
      <Button variant="ghost" size="sm" onClick={open}>
        {actionOf("open").verb}
        <Kbd>↵</Kbd>
      </Button>
      {stoppable ? (
        <Button variant="ghost" size="sm" onClick={() => hosts.onKill!(item.job!.id)}>
          {actionOf("kill").verb}
          <Kbd>{keyFor("kill")}</Kbd>
        </Button>
      ) : null}
    </>
  );
}

export function Cockpit({
  filter,
  onFilter,
  state,
  now,
  picked,
  nowViews,
  nows,
  ...hosts
}: Hosts & {
  /** Which of the three the panel shows. */
  filter: DashboardTab;
  onFilter: (filter: DashboardTab) => void;
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
  nows?: Readonly<Record<string, NowView>> | undefined;
}) {
  const [answered, setAnswered] = useState<ReadonlySet<string>>(new Set());
  const calls = useItems("command-central", state, picked, nowViews, hosts, answered);
  const running = useItems("running", state, picked, nowViews, hosts, answered);
  const over = useItems("done", state, picked, nowViews, hosts, answered);
  const sessions = useSessions();
  // Your move is what needs the owner; Active leads with that, then what runs, a call standing in for
  // the Job or Session it is about; Done is what is over, and what landed on the merge line is no tile.
  const instruments = useMemo(() => {
    if (filter === "command-central") return calls;
    if (filter === "done") return over.filter((one) => !one.key.startsWith("line:"));
    const read = overviewListsOf(state.jobs, picked);
    const unknown = read.undrawable.map(
      (job): Item => ({ key: job.id, owner: job.id, job, icon: CircleDashed, kind: job.status, title: titleOf(job), fact: job.status, hue: "queued", where: job.handle, body: [["Status", job.status]], acts: () => null }),
    );
    const owned = new Set(calls.flatMap((one) => (one.owner === undefined ? [] : [one.owner])));
    return [...calls, ...running.filter((one) => !one.key.startsWith("line:") && !owned.has(one.owner ?? one.key)), ...unknown];
  }, [filter, state, picked, calls, running, over]);
  const [view, setView] = useCockpitView();
  const [field, setField] = useState({ width: 900, height: 560 });
  const sky = useMemo(
    () =>
      skyOf(
        instruments,
        sessions,
        viewsOf(state),
        (root) => state.holds.repositories?.find((one) => one.root === root)?.manifest?.id,
        (manifest) => state.holds.repositories?.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest,
        field,
      ),
    [instruments, sessions, state, field],
  );
  const nowsHeld = useContext(Nows);

  // Calls put off for later, in the order they were; and one brought back to the front by choice.
  const [put, setPut] = useState<readonly string[]>([]);
  const [forward, setForward] = useState<string>();
  const [leaving, setLeaving] = useState<"later" | "sent">();
  const held = new Set(calls.map((one) => one.key));
  const waiting = put.filter((key) => key !== forward && held.has(key));
  const ahead = calls.filter((one) => !waiting.includes(one.key));
  const front = ahead.find((one) => one.key === forward) ?? ahead[0];
  const edge: Item[] = [...ahead.filter((one) => one !== front), ...waiting.map((key) => calls.find((one) => one.key === key)!)];

  /** Keys pressed while a card is leaving, said again once it has gone. */
  const typed = useRef<KeyboardEventInit[]>([]);
  /** Plays the card's exit, then applies what it was leaving for. */
  const leave = (how: "later" | "sent", apply: () => void) => {
    if (stillness()) return apply();
    setLeaving(how);
    window.setTimeout(() => {
      setLeaving(undefined);
      apply();
      const again = typed.current.splice(0);
      window.setTimeout(() => again.forEach((init) => document.body.dispatchEvent(new KeyboardEvent("keydown", { ...init, bubbles: true, cancelable: true }))), 50);
    }, 200);
  };
  const finish = () =>
    front === undefined
      ? undefined
      : leave("sent", () => {
          setAnswered((was) => new Set([...was, front.key]));
          setForward(undefined);
        });
  const later = () =>
    front === undefined
      ? undefined
      : leave("later", () => {
          setPut((was) => [...was.filter((key) => key !== front.key), front.key]);
          setForward(undefined);
        });
  const recall = (key: string) => {
    setPut((was) => was.filter((one) => one !== key));
    setForward(key);
  };

  // The cursor on the glass.
  const [selected, setSelected] = useState<string>();
  const at = Math.max(0, instruments.findIndex((one) => one.key === selected));
  const current = instruments[at];
  useCursor(current?.job, hosts.onCursor);
  const grid = useRef<HTMLUListElement>(null);

  // A call arriving takes the keys: a field that was holding them, empty, lets go.
  useEffect(() => {
    const field = document.activeElement;
    if (front !== undefined && field instanceof HTMLInputElement && field.value === "") field.blur();
  }, [front?.key]);

  // j, k, Enter, o and x are the Board's, as every list reads them; the rest are this screen's.
  useBoardKeys(instruments, at, setSelected, hosts, front === undefined && leaving === undefined);
  const answering = useRef<CardKeys | undefined>(undefined);
  useListKeydown((event) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const moves = ["j", "k", "ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key);
    if (event.repeat && !moves) return;
    if (holdsText(event.target)) {
      // The dispatch bar is always empty: Escape or Down hands the keys back to the glass.
      const field = event.target;
      if (field instanceof HTMLInputElement && field.value === "" && (event.key === "Escape" || event.key === "ArrowDown")) {
        event.preventDefault();
        field.blur();
      }
      return;
    }
    if (leaving !== undefined) {
      // Enter is not said again: a second press must never send the next call.
      if (event.key !== "Enter") typed.current.push({ key: event.key, code: event.code, shiftKey: event.shiftKey });
      return;
    }
    const claim = () => event.preventDefault();

    if (front !== undefined) {
      const card = answering.current;
      if (card === undefined) return;
      if (/^[1-9]$/.test(event.key)) return claim(), card.pickNumber(Number(event.key));
      if (card.pickKey(event.key)) return claim();
      switch (event.key) {
        case "e":
          return card.expand === undefined ? undefined : (claim(), card.expand());
        case "ArrowDown":
        case "j":
          return claim(), card.step(1);
        case "ArrowUp":
        case "k":
          return claim(), card.step(-1);
        case "Enter":
          // A focused button fires itself on Enter.
          if ((event.target as HTMLElement).tagName === "BUTTON") return;
          return card.canSend ? (claim(), card.send()) : undefined;
        case "l":
        case "Escape":
          return claim(), later();
        case "o":
          return card.open === undefined ? undefined : (claim(), card.open());
      }
      return;
    }

    if (event.key === "m") return claim(), setView(view === "map" ? "grid" : "map");
    if (view === "map" && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
      const next = nearest(sky.stars, current?.key ?? "", event.key as "ArrowLeft");
      return next === undefined ? undefined : (claim(), setSelected(next));
    }
    const cols = grid.current === null ? 1 : Math.max(1, getComputedStyle(grid.current).gridTemplateColumns.split(" ").length);
    const to = (by: number) => {
      const next = instruments[Math.min(instruments.length - 1, Math.max(0, at + by))];
      if (next !== undefined) setSelected(next.key);
    };
    switch (event.key) {
      case "ArrowRight":
        return claim(), to(1);
      case "ArrowLeft":
        return claim(), to(-1);
      case "ArrowDown":
        return claim(), to(cols);
      case "ArrowUp":
        return claim(), to(-cols);
      case "w":
        return edge[0] === undefined ? undefined : (claim(), recall(edge[0].key));
    }
  });

  // Calls that have stood behind another: when one comes to the front it slides up from there.
  const seen = useRef(new Set<string>());
  edge.forEach((one) => seen.current.add(one.key));
  const frame = front?.hue ?? edge[0]?.hue;

  const panel =
    filter === "command-central" || current === undefined || current.decisions !== undefined
      ? undefined
      : current.job === undefined
        ? undefined
        : nowPanelOf(nowsHeld?.[current.job.id], { onOpenJob: hosts.onOpen, onSaid: () => {} });
  const pane = filter !== "command-central" && current !== undefined;

  return (
    <div className="armada-cockpit" data-hue={frame} data-settles>
      <section className="armada-view" aria-label="Dashboard" data-hue={frame} data-waiting={frame === undefined ? undefined : ""}>
        <header className="armada-view__band">
          <span className="armada-view__filters">
            <Tooltip label="Previous filter">
              <Kbd>{TAB_KEYS.previous}</Kbd>
            </Tooltip>
            <Tabs items={DASHBOARD_TABS.map((one) => ({ id: one.id, label: one.label }))} value={filter} onChange={(id) => onFilter(id as DashboardTab)} />
            <Tooltip label="Next filter">
              <Kbd>{TAB_KEYS.next}</Kbd>
            </Tooltip>
          </span>
          <Horizon state={state} />
          <span className="armada-view__keys">
            <span className="armada-view__toggle" role="group" aria-label="View">
              <Tooltip label="Grid">
                <Button variant="ghost" size="sm" iconOnly aria-label="Grid" aria-pressed={view === "grid"} onClick={() => setView("grid")}>
                  <LayoutDashboard size={16} aria-hidden="true" />
                </Button>
              </Tooltip>
              <Tooltip label="Map">
                <Button variant="ghost" size="sm" iconOnly aria-label="Map" aria-pressed={view === "map"} onClick={() => setView("map")}>
                  <Waypoints size={16} aria-hidden="true" />
                </Button>
              </Tooltip>
              <Tooltip label="Switch between grid and map">
                <Kbd>m</Kbd>
              </Tooltip>
            </span>
            <Tooltip label={actionOf("move_focus").verb}>
              <span>
                <Kbd>j</Kbd> <Kbd>k</Kbd>
              </span>
            </Tooltip>
            <Tooltip label="Keys">
              <Kbd>?</Kbd>
            </Tooltip>
          </span>
        </header>
        <div className="armada-view__stage">
          <div className="armada-view__body" data-pane={pane || undefined} data-view={view} style={front === undefined && edge.length > 0 ? { paddingBottom: `calc(var(--space-6) * ${Math.min(3, edge.length)} + var(--space-8))` } : undefined}>
            {view === "map" ? (
              <div className="armada-view__map" data-recessed={front === undefined || leaving !== undefined ? undefined : ""}>
                <FleetMap sky={sky} field={field} onField={setField} selected={current?.key} onSelect={setSelected} />
                {current === undefined ? null : (
                  <div className="armada-view__picked">
                    <span>{current.title}</span>
                    <TileActs item={current} hosts={hosts} />
                  </div>
                )}
              </div>
            ) : (
              <ul ref={grid} className="armada-tiles armada-view__grid" role="listbox" aria-label="Tiles" tabIndex={0} data-recessed={front === undefined || leaving !== undefined ? undefined : ""}>
                {instruments.map((one) => (
                  <FleetTile key={one.key} item={one} now={now} selected={one.key === current?.key} onSelect={setSelected} extra={<TileActs item={one} hosts={hosts} />} />
                ))}
              </ul>
            )}
            {pane ? (
              <div className="armada-view__pane">
                <CallPane item={current} now={now} workflows={state.holds.workflows} {...(panel === undefined ? {} : { nowPanel: panel })} onDone={() => setAnswered((was) => new Set([...was, current.key]))} onOpenSession={hosts.onOpenSession} onOpenJob={hosts.onOpen} onOpenLink={hosts.onOpenLink} />
              </div>
            ) : null}
          </div>
          {front !== undefined ? (
            <div className="armada-cockpit__scrim" data-hue={front.hue}>
              <div className="armada-stack" style={{ ["--behind" as string]: Math.min(3, edge.length) }}>
                <Behind edge={edge} put={waiting} recall={recall} />
                <CallCard key={front.key} item={front} now={now} nowing={front.job === undefined ? undefined : nowPanelOf(nows?.[front.job.id], { onOpenJob: hosts.onOpen, onSaid: () => {} })} state={state} hosts={hosts} finish={finish} later={later} leaving={leaving} from={seen.current.has(front.key) ? "stack" : undefined} answering={answering} />
              </div>
            </div>
          ) : edge.length === 0 ? null : (
            <div className="armada-tray">
              <div className="armada-stack" data-tray style={{ ["--behind" as string]: Math.min(3, edge.length) }}>
                <Behind edge={edge} put={waiting} recall={recall} />
              </div>
            </div>
          )}
        </div>
      </section>
    </div>
  );
}

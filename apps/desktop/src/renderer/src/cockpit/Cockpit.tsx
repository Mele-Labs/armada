// The Dashboard's panel as a cockpit: a top bar with three filters (Your move, Active, Done) and the
// way to read them, a grid of tiles or a map of stars, and a pane for the one picked. A call that
// needs the owner comes forward over the panel whatever the filter, centred and answerable from the
// keyboard, the calls behind it stacked like a deck; `l` puts the one in front at the back and `d`
// dismisses it for good.

import { useContext, useEffect, useMemo, useRef, useState } from "react";
import { Box, CircleDashed, GitMerge, LayoutGrid, Orbit, SquareTerminal } from "lucide-react";
import { Button, Kbd, Tabs, Tooltip, actionOf, isPressed, keyFor, pressedDigit, pressedSlot } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowView } from "@armada/jobs/draft/now";
import { nowPanelOf } from "@armada/jobs";
import { isTerminal, titleOf } from "@armada/screens";
import { overviewListsOf, type DashboardTab } from "@armada/overview";
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
import { useLayout } from "@armada/shell";
import { tabKeys, useFilters } from "./keys";
import { dotsOf, type Dot } from "./horizon";
import { sessionIdOf } from "./waiting";
import { nearest, skyOf } from "./map-layout";
import { useCockpitView } from "./view";
import "./cockpit.css";

/**
 * How long an answered call is held out of the deck if Fleet still carries it. **Fleet answers the POST
 * before it has re-derived what a Session waits on or whether a Job asks**, so the next update can carry
 * the call again; a call is held gone until an update stops carrying it, and this is the longest that
 * holds before the call is allowed back (with no word, since nothing was refused). An object so a
 * test can shorten it.
 */
export const holdFor = { ms: 30_000 };

const NONE: ReadonlySet<string> = new Set();

/** Whether motion is off: nothing waits for an exit that will not play. */
const stillness = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** What a dot says on hover: the pull request or the landing, row by row, each drawn only where its fact is. */
function DotCard({ dot }: { dot: Dot }) {
  const { card } = dot;
  const State = dot.icon;
  const Owner = card.owner === undefined ? null : card.owner.kind === "job" ? Box : SquareTerminal;
  return (
    <span className="armada-horizon-card" data-state={dot.state}>
      <span className="armada-horizon-card__head">
        <span className="armada-horizon-card__number">{card.heading}</span>
        {card.title === undefined ? null : <span>{card.title}</span>}
      </span>
      <span className="armada-horizon-card__row armada-horizon-card__branch">{card.branch === card.heading ? null : card.branch}</span>
      <span className="armada-horizon-card__row armada-horizon-card__state">
        {State === null ? null : <State size={14} aria-hidden="true" />}
        {card.says}
      </span>
      {card.check === undefined ? null : <span className="armada-horizon-card__row">{card.check}</span>}
      {card.place === undefined ? null : <span className="armada-horizon-card__row">{dot.card.heading.startsWith("#") ? "Place" : "Turn"} {card.place}</span>}
      {Owner === null || card.owner === undefined ? null : (
        <span className="armada-horizon-card__row">
          <Owner size={14} aria-hidden="true" />
          {card.owner.title}
        </span>
      )}
    </span>
  );
}

/**
 * The merge line as the glass's horizon, a band along the foot of the panel under the grid or the
 * map: dots moving toward main's light at the right end, nearest main first. Fleet's landings come
 * first, then the pull requests in the forge's merge queue, then the open ones. A dot is coloured
 * by its state, solid where a queue holds it and a ring where it is open, and only a running one
 * pulses. Hovering or focusing one opens its card; pressing it opens the pull request or the Job.
 */
function Horizon({ state, sessions, hosts }: { state: BridgeState; sessions: readonly Session[]; hosts: Hosts }) {
  const views = viewsOf(state);
  if (views.length === 0) return null;
  return (
    <footer className="armada-view__horizon" aria-label="Merge line">
      {views.map((view) => (
        <div key={view.root} className="armada-view__line">
          <GitMerge size={16} aria-hidden="true" />
          <ol className="armada-view__belt">
            {dotsOf(view, state.mergeLines, views, sessions).map((dot) => {
              const act = dot.act;
              return (
                <li key={dot.key} className="armada-view__dot-item">
                  <Tooltip label={<DotCard dot={dot} />} card asChild>
                    {act === undefined ? (
                      <span className="armada-view__dot" data-state={dot.state} data-queued={dot.queued || undefined} role="img" aria-label={dot.tip} />
                    ) : (
                      <button
                        type="button"
                        className="armada-view__dot"
                        data-state={dot.state}
                        data-queued={dot.queued || undefined}
                        aria-label={dot.tip}
                        onClick={() => (act.kind === "link" ? hosts.onOpenLink(act.url) : hosts.onOpen(act.id))}
                      />
                    )}
                  </Tooltip>
                </li>
              );
            })}
          </ol>
          <Tooltip label={view.hub?.main?.state === "red" ? "Main is red" : "Main is green"}>
            <span className="armada-view__main" data-red={view.hub?.main?.state === "red" || undefined} />
          </Tooltip>
        </div>
      ))}
    </footer>
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
            {index === 0 ? <Kbd>{keyFor("call_recall")}</Kbd> : null}
          </button>
        );
      })}
    </>
  );
}

/** The acts of the tile under the cursor, each with the key that does it. */
function TileActs({ item, hosts }: { item: Item; hosts: Hosts }) {
  const open = () => (item.job === undefined ? hosts.onOpenSession(sessionIdOf(item.key)!) : hosts.onOpen(item.job.id));
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
  // Calls answered or dismissed, by key and when: held out of every list until the data stops carrying them.
  const [holds, setHolds] = useState<ReadonlyMap<string, number>>(new Map());
  const hold = (key: string) => setHolds((was) => new Map(was).set(key, Date.now()));
  const sessions = useSessions();
  const carriedCalls = useItems("command-central", state, picked, nowViews, hosts, NONE);
  const carriedRunning = useItems("running", state, picked, nowViews, hosts, NONE);
  const carriedOver = useItems("done", state, picked, nowViews, hosts, NONE);
  const calls = useMemo(() => carriedCalls.filter((one) => !holds.has(one.key)), [carriedCalls, holds]);
  const running = useMemo(() => carriedRunning.filter((one) => !holds.has(one.key)), [carriedRunning, holds]);
  const over = useMemo(() => carriedOver.filter((one) => !holds.has(one.key)), [carriedOver, holds]);
  // A hold lets go when an update no longer carries its call, or after `holdFor`. Not while the Board is
  // empty: a resync carries nothing, and letting go then would bring the call back to be dropped again.
  const board = state.jobs.length > 0 || viewsOf(state).length > 0 || sessions.length > 0;
  useEffect(() => {
    if (holds.size === 0) return;
    const carried = new Set([...carriedCalls, ...carriedRunning, ...carriedOver].map((one) => one.key));
    const expired = (since: number) => Date.now() - since >= holdFor.ms;
    const gone = [...holds].filter(([key, since]) => expired(since) || (board && !carried.has(key)));
    if (gone.length > 0) return setHolds((was) => new Map([...was].filter(([key]) => !gone.some(([one]) => one === key))));
    const next = Math.min(...holds.values()) + holdFor.ms - Date.now();
    const timer = window.setTimeout(() => setHolds((was) => new Map(was)), Math.max(0, next));
    return () => window.clearTimeout(timer);
  }, [holds, carriedCalls, carriedRunning, carriedOver, board]);
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
          hold(front.key);
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
    // Every act's keys are the keymap's, which reads modifiers exactly. The arrows that move across the
    // glass are spatial rather than an act, and stay bare keys.
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    const arrow = bare && ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(event.key);
    const step = pressedSlot("move_focus", event);
    const moves = step !== -1 || arrow;
    if (event.repeat && !moves) return;
    if (holdsText(event.target)) {
      // The dispatch bar is always empty: Escape or Down hands the keys back to the glass.
      const field = event.target;
      if (field instanceof HTMLInputElement && field.value === "" && (isPressed("close", event) || (bare && event.key === "ArrowDown"))) {
        event.preventDefault();
        field.blur();
      }
      return;
    }
    if (leaving !== undefined) {
      // Enter is not said again: a second press must never send the next call.
      if (bare && !isPressed("call_send", event)) typed.current.push({ key: event.key, code: event.code, shiftKey: event.shiftKey });
      return;
    }
    const claim = () => event.preventDefault();

    if (front !== undefined) {
      const card = answering.current;
      if (card === undefined) return;
      const digit = pressedDigit("call_pick", event);
      if (digit !== null) return claim(), card.pickNumber(digit);
      if (card.pickKey(event)) return claim();
      if (isPressed("call_reply", event)) {
        const field = document.querySelector<HTMLTextAreaElement>(".armada-callcard__reply textarea");
        return field === null ? undefined : (claim(), field.focus());
      }
      if (isPressed("call_expand", event)) return card.expand === undefined ? undefined : (claim(), card.expand());
      if (step === 0 || step === 2) return claim(), card.step(1);
      if (step === 1 || step === 3) return claim(), card.step(-1);
      if (isPressed("call_send", event)) {
        // A button on the card fires itself on Enter; one elsewhere (the filter just pressed) is not the answer.
        if ((event.target as HTMLElement).tagName === "BUTTON" && document.querySelector(".armada-callcard")?.contains(event.target as Node) === true) return;
        return card.canSend ? (claim(), card.send()) : undefined;
      }
      // Nothing else on the card acts while an answer is out, or the answer would clear the wrong one.
      if (isPressed("call_later", event) || isPressed("close", event)) return claim(), card.pending === undefined ? later() : undefined;
      if (isPressed("call_dismiss", event)) return claim(), card.dismiss();
      if (isPressed("open", event)) return card.open === undefined ? undefined : (claim(), card.open());
      return;
    }

    if (isPressed("dashboard_view", event)) return claim(), setView(view === "map" ? "grid" : "map");
    if (view === "map" && arrow) {
      const next = nearest(sky.stars, current?.key ?? "", event.key as "ArrowLeft");
      return next === undefined ? undefined : (claim(), setSelected(next));
    }
    const cols = grid.current === null ? 1 : Math.max(1, getComputedStyle(grid.current).gridTemplateColumns.split(" ").length);
    const to = (by: number) => {
      const next = instruments[Math.min(instruments.length - 1, Math.max(0, at + by))];
      if (next !== undefined) setSelected(next.key);
    };
    if (arrow) {
      switch (event.key) {
        case "ArrowRight":
          return claim(), to(1);
        case "ArrowLeft":
          return claim(), to(-1);
        case "ArrowDown":
          return claim(), to(cols);
        case "ArrowUp":
          return claim(), to(-cols);
      }
    }
    if (isPressed("call_recall", event)) return edge[0] === undefined ? undefined : (claim(), recall(edge[0].key));
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
  const filters = useFilters(filter);
  const mergeLine = useLayout("dashboard.panels").shown.some((one) => one.id === "merge-line");

  return (
    <div className="armada-cockpit" data-hue={frame} data-settles>
      <section className="armada-view" aria-label="Dashboard" data-hue={frame} data-waiting={frame === undefined ? undefined : ""}>
        <header className="armada-view__band">
          <span className="armada-view__filters">
            <Tooltip label="Previous filter">
              <Kbd>{tabKeys().previous}</Kbd>
            </Tooltip>
            <Tabs items={filters} value={filter} onChange={(id) => onFilter(id as DashboardTab)} />
            <Tooltip label="Next filter">
              <Kbd>{tabKeys().next}</Kbd>
            </Tooltip>
          </span>
          <span className="armada-view__keys">
            <span className="armada-view__toggle" role="group" aria-label="View">
              <Tooltip label="Grid">
                <Button variant="ghost" size="sm" iconOnly aria-label="Grid" aria-pressed={view === "grid"} onClick={() => setView("grid")}>
                  <LayoutGrid size={16} aria-hidden="true" />
                </Button>
              </Tooltip>
              <Tooltip label="Map">
                <Button variant="ghost" size="sm" iconOnly aria-label="Map" aria-pressed={view === "map"} onClick={() => setView("map")}>
                  <Orbit size={16} aria-hidden="true" />
                </Button>
              </Tooltip>
              <Tooltip label={actionOf("dashboard_view").verb}>
                <Kbd>{keyFor("dashboard_view")}</Kbd>
              </Tooltip>
            </span>
            <Tooltip label={actionOf("move_focus").verb}>
              <span>
                <Kbd>j</Kbd> <Kbd>k</Kbd>
              </span>
            </Tooltip>
            <Tooltip label={actionOf("key_sheet").verb}>
              <Kbd>{keyFor("key_sheet")}</Kbd>
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
                <CallPane item={current} now={now} workflows={state.holds.workflows} {...(panel === undefined ? {} : { nowPanel: panel })} onDone={() => hold(current.key)} onOpenSession={hosts.onOpenSession} onOpenJob={hosts.onOpen} onOpenLink={hosts.onOpenLink} />
              </div>
            ) : null}
          </div>
          {front !== undefined ? (
            <div className="armada-cockpit__scrim" data-hue={front.hue}>
              <div className="armada-stack" style={{ ["--behind" as string]: Math.min(3, edge.length) }}>
                <Behind edge={edge} put={waiting} recall={recall} />
                <CallCard key={front.key} item={front} now={now} nowing={front.job === undefined ? undefined : nowPanelOf(nows?.[front.job.id], { onOpenJob: hosts.onOpen, onSaid: () => {} })} state={state} hosts={hosts} finish={finish} later={later} dismiss={finish} leaving={leaving} from={seen.current.has(front.key) ? "stack" : undefined} answering={answering} />
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
        {mergeLine ? <Horizon state={state} sessions={sessions} hosts={hosts} /> : null}
      </section>
    </div>
  );
}

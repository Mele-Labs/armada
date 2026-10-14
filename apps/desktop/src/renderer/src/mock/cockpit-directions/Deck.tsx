// The Cockpit as a flight deck. A top bar (the real title bar, with the places as tabs in its middle), a left
// column of every Job and Session with the plan's usage under it, a dynamic area that is the map until something
// is picked and then that thing (a Job's canvas, a Session's thread, a bay, a pull request, a call), the merge
// queue as one dot a pull request, the bays on the right, and along the foot the flight controls and the calls
// that are incoming. Esc returns to the map or timeline exactly as it was left. The state is held in
// `deck-model.ts`, so the top bar, the body and the keys are one thing. Mock only. proto/cockpit-flight-deck.

import { useEffect, useLayoutEffect, useMemo, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Box, ChartNoAxesGantt, GitMerge, Megaphone, Orbit, Plane, ShieldCheck, SquareTerminal } from "lucide-react";
import { Button, DropdownMenu, Kbd, Tabs, Tooltip } from "@armada/components";
import { usageLevel } from "@armada/components/src/usage";
import { stateOf as bayStateOf } from "@armada/components/src/compositions/PoolSlots/PoolTile";
import type { WorktreeSlot } from "@armada/protocol";
import { FIND_A_REPOSITORY, SURFACE, repositoryEntries, rootOf, useRecentRepositories } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import { age, useItems, type Item } from "../../Dashboard";
import type { AppSeamProps } from "../../cockpit/app-seam";
import { useAutopilotLane } from "../../cockpit/AutopilotLane";
import type { CockpitProps } from "../../cockpit/body-seam";
import { holdFor } from "../../cockpit/Cockpit";
import { dotsOf } from "../../cockpit/horizon";
import { skyOf, type Field } from "../../cockpit/map-layout";
import { Timeline } from "../../cockpit/Timeline";
import { sessionIdOf } from "../../cockpit/waiting";
import { viewsOf } from "../../merge-line";
import { usePlanUsage } from "../../plan-usage";
import { useSessions } from "../../sessions-draft";
import { CallPane, ChecksPane, JobPane, PrPane, SessionPane, SlotPane } from "./deck-panes";
import {
  arrivedLate,
  deckNow,
  incomingOf,
  listOf,
  moveCursor,
  noteCalls,
  release,
  select,
  selOfPr,
  selOfSubject,
  setHere,
  setView,
  settle,
  slotKey,
  toggleNeeds,
  toggleOnly,
  useDeck,
  visit,
} from "./deck-model";
import { MarkGlyph, useShipActs } from "./shared";
import { useShip, type Pr, type Ship, type Subject } from "./ship";
import { toneOf } from "./tabs-model";
import { StarMap } from "./StarMap";
import "./deck.css";

const NONE: ReadonlySet<string> = new Set();

const PLACES = [
  { id: "cockpit", label: "Cockpit" },
  { id: "jobs", label: "Jobs" },
  { id: "sessions", label: "Sessions" },
  { id: "system", label: "System" },
];
/** The Machine pages. Fleet's own page is Doctor. */
const SYSTEM = [
  { id: SURFACE.kit, label: "Kit" },
  { id: SURFACE.mods, label: "Mods" },
  { id: SURFACE.settings, label: "Settings" },
  { id: SURFACE.guides, label: "Guides" },
  { id: SURFACE.workflows, label: "Workflows" },
  { id: SURFACE.doctor, label: "Doctor" },
];

/** Which place the window is on, read off the app's own state. */
function placeOf(showing: string): string {
  if (showing === SURFACE.overview) return "cockpit";
  if (showing === SURFACE.jobs) return "jobs";
  if (showing === SURFACE.sessions) return "sessions";
  return SYSTEM.some((one) => one.id === showing) ? "system" : "";
}

/** The real title bar's two ends, found once it is drawn. The tabs and the wordmark are portalled into them. */
function useTitleBar(): { start: HTMLElement | undefined; centre: HTMLElement | undefined } {
  const [hosts, setHosts] = useState<{ start: HTMLElement | undefined; centre: HTMLElement | undefined }>({ start: undefined, centre: undefined });
  useLayoutEffect(() => {
    const find = () => {
      const start = document.querySelector<HTMLElement>(".armada-title-bar__start") ?? undefined;
      const centre = document.querySelector<HTMLElement>(".armada-title-bar__center") ?? undefined;
      setHosts((was) => (was.start === start && was.centre === centre ? was : { start, centre }));
    };
    find();
    const watch = new MutationObserver(find);
    watch.observe(document.body, { childList: true, subtree: true });
    return () => watch.disconnect();
  }, []);
  return hosts;
}

// ---- The chrome ---------------------------------------------------------------------------------------

/**
 * What the shell draws at the top of its mount on every page: the places in the title bar, the Machine pages under
 * them while System is the place, and the keys, which are the window's wherever he is.
 */
export function DeckChrome(props: AppSeamProps) {
  const ship = useShip(props, false);
  const deck = useDeck();
  noteCalls(ship.calls);
  const here = props.showing === SURFACE.overview && props.openJobId === null;
  useEffect(() => setHere(here), [here]);
  const place = placeOf(props.showing);
  const bar = useTitleBar();

  // An answered call is held out until Fleet stops carrying it, or `holdFor` has passed.
  useEffect(() => {
    if (deck.held.size === 0) return;
    const live = new Set(ship.calls.map((one) => one.key));
    const gone = [...deck.held].filter(([key, since]) => !live.has(key) || Date.now() - since >= holdFor.ms).map(([key]) => key);
    if (gone.length > 0) return release(gone);
    const next = Math.min(...deck.held.values()) + holdFor.ms - Date.now();
    const timer = window.setTimeout(() => release([]), Math.max(0, next));
    return () => window.clearTimeout(timer);
  }, [deck.held, ship.calls]);

  const go = (id: string) => {
    if (id === place) return;
    if (id === "cockpit") props.goTo(SURFACE.overview);
    else if (id === "jobs") props.goTo(SURFACE.jobs);
    else if (id === "sessions") props.goTo(SURFACE.sessions);
    else props.goTo(SURFACE.kit);
  };

  const incoming = incomingOf(ship.calls, deck.held, deck.visited);
  const listed = useMemo(() => listOf(ship, deck), [ship, deck.only, deck.needs, deck.held]);
  const openNext = () => {
    const next = incoming[0];
    if (next === undefined) return;
    visit(next.key);
    select({ kind: "call", key: next.key });
    if (!deckNow().here) props.goTo(SURFACE.overview);
  };

  useListKeydown((event) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const state = deckNow();
    if (holdsText(event.target)) {
      // The first Esc leaves the box, the next one leaves the pane.
      if (event.key === "Escape" && state.here && state.sel !== undefined && state.sel.kind !== "call") {
        event.preventDefault();
        (event.target as HTMLElement).blur();
      }
      return;
    }
    if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
    if (event.key === "w") {
      event.preventDefault();
      return openNext();
    }
    if (!state.here || state.sel?.kind === "call") return;
    const target = event.target as HTMLElement;
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp": {
        event.preventDefault();
        const at = listed.findIndex((one) => one.key === state.cursor);
        const next = listed[Math.min(listed.length - 1, Math.max(0, at < 0 ? (event.key === "ArrowDown" ? 0 : listed.length - 1) : at + (event.key === "ArrowDown" ? 1 : -1)))];
        if (next === undefined) return;
        moveCursor(next.key);
        window.requestAnimationFrame(() => document.querySelector(`[data-row="${next.key}"]`)?.scrollIntoView?.({ block: "nearest" }));
        return;
      }
      case "Enter": {
        if (target.tagName === "BUTTON" || target.tagName === "A") return;
        const row = listed.find((one) => one.key === state.cursor);
        if (row === undefined) return;
        event.preventDefault();
        select(selOfSubject(row));
        return;
      }
      case "Escape":
        if (state.sel !== undefined) {
          event.preventDefault();
          select(undefined);
        }
        return;
      case "m":
        event.preventDefault();
        return setView("map");
      case "t":
        event.preventDefault();
        return setView("timeline");
    }
  });

  return (
    <>
      {bar.start === undefined ? null : createPortal(<span className="deck-wordmark">Armada</span>, bar.start)}
      {bar.centre === undefined
        ? null
        : createPortal(
            <nav className="deck-places" aria-label="Places">
              <Tabs items={PLACES.map((one) => (one.id === "cockpit" && !here && incoming.length > 0 ? { ...one, alert: "A call is waiting" } : one))} value={place} onChange={go} />
            </nav>,
            bar.centre,
          )}
      <div className="deck-chrome">
        {place !== "system" ? null : (
          <nav className="deck-system" aria-label="System">
            <Tabs items={SYSTEM} value={props.showing} onChange={(id) => props.goTo(id)} />
          </nav>
        )}
      </div>
    </>
  );
}

// ---- The left column ----------------------------------------------------------------------------------

function List({ subjects, cursor, selected, onPick }: { subjects: readonly Subject[]; cursor: string | undefined; selected: string | undefined; onPick: (subject: Subject) => void }) {
  return (
    <section className="deck-panel deck-list" aria-label="Jobs and Sessions">
      <header className="deck-band">Jobs and Sessions</header>
      <ul className="deck-list__rows">
        {subjects.map((one) => {
          const Kind = one.kind === "job" ? Box : SquareTerminal;
          return (
            <li key={one.key}>
              <button type="button" className="deck-row" data-row={one.key} data-tone={toneOf(one)} data-selected={selected === one.key || undefined} data-cursor={cursor === one.key || undefined} aria-label={one.title} aria-current={selected === one.key || undefined} onClick={() => onPick(one)}>
                <MarkGlyph mark={{ ...one.mark, tone: toneOf(one) }} />
                <span className="deck-row__title">{one.title}</span>
                <Tooltip label={one.kind === "job" ? "Job" : "Session"}>
                  <span className="deck-row__kind" role="img" aria-label={one.kind === "job" ? "Job" : "Session"}>
                    <Kind size={12} aria-hidden />
                  </span>
                </Tooltip>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** The plan's windows as bars. A bar is gray until it is nearly full. */
function Usage() {
  const usage = usePlanUsage() ?? [];
  const NAMES: Record<string, { short: string; name: string }> = { five_hour: { short: "5h", name: "5-hour limit" }, seven_day: { short: "wk", name: "Weekly limit" } };
  const drawn = usage.flatMap((one) => (NAMES[one.kind] === undefined ? [] : [{ one, ...NAMES[one.kind]! }]));
  return (
    <section className="deck-panel deck-usage" aria-label="Usage">
      <header className="deck-band">Usage</header>
      <div className="deck-usage__bars">
        {drawn.map(({ one, short, name }) => {
          const percent = Math.round(one.percentUsed);
          return (
            <Tooltip key={one.kind} label={`${name} ${percent}%`} asChild>
              <div className="deck-meter" role="meter" aria-label={`${name} ${percent}%`} aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent} data-level={usageLevel(percent)}>
                <span className="deck-meter__label">{short}</span>
                <span className="deck-meter__track">
                  <span className="deck-meter__fill" style={{ width: `${Math.min(100, Math.max(0, percent))}%` }} />
                </span>
              </div>
            </Tooltip>
          );
        })}
      </div>
    </section>
  );
}

// ---- The dynamic area ---------------------------------------------------------------------------------

const subjectKeyOf = (item: Item): string | undefined => {
  if (item.job !== undefined) return `job:${item.job.id}`;
  const id = sessionIdOf(item.key);
  return id === undefined ? undefined : `session:${id}`;
}

function MapView({ props, ship, listed, cursor }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; cursor: string | undefined }) {
  const sessions = useSessions();
  const running = useItems("running", props.state, props.picked, props.nowViews, props, NONE);
  const [field, setField] = useState<Field>({ width: 900, height: 560 });
  const shown = useMemo(() => new Set(listed.map((one) => one.key)), [listed]);
  const items = useMemo(() => {
    const owned = new Set(ship.calls.flatMap((one) => (one.owner === undefined ? [] : [one.owner])));
    return [...ship.calls, ...running.filter((one) => !one.key.startsWith("line:") && !owned.has(one.owner ?? one.key))].filter((one) => {
      const key = subjectKeyOf(one);
      return key !== undefined && shown.has(key);
    });
  }, [ship.calls, running, shown]);
  const sky = useMemo(
    () =>
      skyOf(
        items,
        sessions,
        viewsOf(props.state),
        (root) => props.state.holds.repositories?.find((one) => one.root === root)?.manifest?.id,
        (manifest) => props.state.holds.repositories?.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest,
        field,
      ),
    [items, sessions, props.state, field],
  );
  const star = sky.stars.find((one) => subjectKeyOf(one.item) === cursor)?.key;
  return (
    <section className="deck-pane" aria-label="Flight map">
      <header className="deck-band deck-pane__head">
        <Orbit size={16} aria-hidden />
        <h2 className="deck-pane__title">Map</h2>
      </header>
      <div className="deck-map">
        <StarMap
          sky={sky}
          field={field}
          onField={setField}
          selected={star}
          onSelect={(key) => {
            const found = sky.stars.find((one) => one.key === key);
            const subject = found === undefined ? undefined : ship.subjects.find((one) => one.key === subjectKeyOf(found.item));
            if (subject !== undefined) select(selOfSubject(subject));
          }}
        />
      </div>
    </section>
  );
}

function TimelineView({ props, ship, listed }: { props: CockpitProps; ship: Ship; listed: readonly Subject[] }) {
  const views = useMemo(() => viewsOf(props.state), [props.state]);
  const shown = useMemo(() => new Set(listed.map((one) => one.key)), [listed]);
  const deck = useDeck();
  const keep = useMemo(
    () => (subject: { kind: string; id?: string }) => (subject.kind === "main" ? deck.only === undefined && !deck.needs : shown.has(`${subject.kind}:${subject.id}`)),
    [shown, deck.only, deck.needs],
  );
  const pick = (key: string) => {
    const subject = ship.subjects.find((one) => one.key === key);
    if (subject !== undefined && subject.kind !== "pr") select(selOfSubject(subject));
  };
  return (
    <section className="deck-pane" aria-label="Fleet timeline">
      <header className="deck-band deck-pane__head">
        <ChartNoAxesGantt size={16} aria-hidden />
        <h2 className="deck-pane__title">Timeline</h2>
      </header>
      <div className="deck-timeline">
        <Timeline
          now={props.now}
          jobs={props.state.jobs}
          views={views}
          filter="running"
          recessed={false}
          keep={keep as never}
          onOpenJob={(id) => pick(`job:${id}`)}
          onOpenSession={(id) => pick(`session:${id}`)}
          onOpenLink={(address) => {
            const pr = ship.prs.find((one) => one.url === address);
            if (pr === undefined) props.onOpenLink(address);
            else select(selOfPr(pr));
          }}
        />
      </div>
    </section>
  );
}

/** The slots the pool serves, narrowed to the picked repository as the ship's are. */
const slotsOf = (props: CockpitProps): readonly WorktreeSlot[] =>
  props.state.held.state === "read" ? (props.state.held.held.slots ?? []).filter((one) => props.picked?.manifest === undefined || one.manifest_id === props.picked.manifest.id) : [];

const repoNameOf = (props: CockpitProps, manifest: string) => props.state.holds.repositories?.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest;

function Stage({ props, ship, listed, acts }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; acts: ReturnType<typeof useShipActs> }) {
  const deck = useDeck();
  const sel = deck.sel;
  const subject = sel === undefined ? undefined : ship.subjects.find((one) => one.key === sel.key && one.kind !== "pr");
  const slot = sel?.kind === "slot" ? slotsOf(props).find((one) => slotKey(one) === sel.key) : undefined;
  const pr = sel?.kind === "pr" ? ship.prs.find((one) => one.key === sel.key) : undefined;
  const call = sel?.kind === "call" ? ship.calls.find((one) => one.key === sel.key) : undefined;
  // What was picked is gone (a Job ended, a call was answered elsewhere): back to the map.
  const checks = sel?.kind === "checks";
  const gone = sel !== undefined && !checks && subject === undefined && slot === undefined && pr === undefined && call === undefined;
  useEffect(() => {
    if (gone) select(undefined);
  }, [gone]);
  const onPr = (one: Pr) => select(selOfPr(one));
  if (sel?.kind === "job" && subject !== undefined) return <JobPane subject={subject} props={props} acts={acts} onPr={onPr} />;
  if (sel?.kind === "session" && subject !== undefined) return <SessionPane subject={subject} props={props} acts={acts} onPr={onPr} />;
  if (slot !== undefined) return <SlotPane slot={slot} repo={repoNameOf(props, slot.manifest_id)} now={props.now} />;
  if (pr !== undefined) return <PrPane pr={pr} owner={ship.subjects.find((one) => one.pr?.key === pr.key)} props={props} acts={acts} />;
  if (call !== undefined) return <CallPane item={call} props={props} />;
  if (checks) return <ChecksPane props={props} ship={ship} />;
  return deck.view === "map" ? <MapView props={props} ship={ship} listed={listed} cursor={deck.cursor} /> : <TimelineView props={props} ship={ship} listed={listed} />;
}

/** The merge queue: one row, one dot for each pull request, each repository's nearest main first, ending at one light for main. No count. */
function MergeQueue({ props, ship, selected }: { props: CockpitProps; ship: Ship; selected: string | undefined }) {
  const sessions = useSessions();
  const views = viewsOf(props.state);
  if (views.length === 0) return null;
  const red = views.filter((view) => view.hub?.main?.state === "red").map((view) => ship.repos.find((one) => one.root === view.root)?.name ?? view.root);
  return (
    <section className="deck-panel deck-queue armada-view__horizon" aria-label="Merge queue">
      <div className="armada-view__line">
        <GitMerge size={16} aria-hidden="true" />
        <ol className="armada-view__belt">
          {views.flatMap((view, at) => [
            ...(at === 0 ? [] : [<li key={`gap:${view.root}`} className="deck-queue__gap" aria-hidden="true" />]),
            ...dotsOf(view, props.state.mergeLines, views, sessions).map((dot) => {
              const pr = ship.prs.find((one) => one.root === view.root && `pull:${one.number}` === dot.key);
              const owner = pr === undefined ? undefined : ship.subjects.find((one) => one.pr?.key === pr.key);
              const tone = pr?.stage === "red" ? "bad" : owner?.mergeable === true ? "ask" : "calm";
              return (
                <li key={`${view.root}:${dot.key}`} className="armada-view__dot-item">
                  <Tooltip label={dot.tip} asChild>
                    {pr === undefined ? (
                      <span className="armada-view__dot" data-state={dot.state} data-queued={dot.queued || undefined} data-tone={tone} role="img" aria-label={dot.tip} />
                    ) : (
                      <button type="button" className="armada-view__dot" data-state={dot.state} data-queued={dot.queued || undefined} data-tone={tone} data-selected={selected === pr.key || undefined} aria-label={dot.tip} onClick={() => select(selOfPr(pr))} />
                    )}
                  </Tooltip>
                </li>
              );
            }),
          ])}
        </ol>
        <Tooltip label={red.length > 0 ? `Main is red in ${red.join(", ")}` : "Main is green"}>
          <span className="armada-view__main" data-red={red.length > 0 || undefined} />
        </Tooltip>
      </div>
    </section>
  );
}

// ---- The right column, and the foot ------------------------------------------------------------------

function Bays({ props, selected }: { props: CockpitProps; selected: string | undefined }) {
  const slots = slotsOf(props);
  return (
    <section className="deck-panel deck-bays" aria-label="Bays">
      <header className="deck-band">Bays</header>
      <ul className="deck-bays__rows">
        {slots.map((slot) => {
          const state = bayStateOf({ slot });
          const held = slot.held;
          const holder = held.state === "job" ? (held.job_title ?? held.job_id) : held.state === "hosted_session" ? (held.session_title ?? held.session_id) : undefined;
          const key = slotKey(slot);
          return (
            <li key={key}>
              <button type="button" className="deck-bay" data-bay={slot.closed === true ? "closed" : state.bay} data-selected={selected === key || undefined} aria-label={`slot-${slot.slot}, ${repoNameOf(props, slot.manifest_id)}, ${slot.closed === true ? "Closed" : state.word}`} onClick={() => select({ kind: "slot", key })}>
                <Tooltip label={slot.closed === true ? "Closed" : state.word}>
                  <span className="deck-bay__state" role="img" aria-label={slot.closed === true ? "Closed" : state.word}>
                    <state.Glyph size={16} aria-hidden />
                  </span>
                </Tooltip>
                <span className="deck-bay__name">slot-{slot.slot}</span>
                <span className="deck-bay__holder">{holder ?? ""}</span>
                <span className="deck-bay__repo">{repoNameOf(props, slot.manifest_id)}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function Pressed({ label, keys, pressed, onPress, children }: { label: string; keys?: string; pressed: boolean; onPress: () => void; children: ReactNode }) {
  return (
    <Tooltip label={label} {...(keys === undefined ? {} : { shortcut: keys })}>
      <Button variant="ghost" size="sm" iconOnly aria-label={label} aria-pressed={pressed} onClick={onPress}>
        {children}
      </Button>
    </Tooltip>
  );
}

function Controls({ props, autopilot }: { props: CockpitProps; autopilot: ReturnType<typeof useAutopilotLane> }) {
  const deck = useDeck();
  const [recent, remember] = useRecentRepositories();
  const control = autopilot.control;
  return (
    <section className="deck-panel deck-controls" aria-label="Flight controls">
      <header className="deck-band">Flight controls</header>
      <div className="deck-controls__row">
        {control === undefined ? null : (
          <Tooltip label={control.state === "off" ? "Autopilot" : `Autopilot, ${control.state}`}>
            <span>
              {props.picked === null ? (
                <DropdownMenu
                  align="end"
                  portal
                  icon={Plane}
                  triggerLabel="Autopilot"
                  searchLabel={FIND_A_REPOSITORY}
                  entries={repositoryEntries(props.state.holds.repositories ?? [], { onlySetUp: true, recent })}
                  onSelect={(chosen) => {
                    const root = rootOf(chosen);
                    remember(root);
                    control.openNext();
                    props.onPickRepository?.(root);
                  }}
                />
              ) : (
                <Button variant="ghost" size="sm" iconOnly aria-label="Autopilot" aria-pressed={control.open} data-autopilot={control.state} onClick={control.toggle}>
                  <Plane size={16} aria-hidden="true" />
                </Button>
              )}
            </span>
          </Tooltip>
        )}
        <span className="deck-controls__group" role="group" aria-label="Show">
          <Pressed label="Jobs only" pressed={deck.only === "jobs"} onPress={() => toggleOnly("jobs")}>
            <Box size={16} aria-hidden />
          </Pressed>
          <Pressed label="Sessions only" pressed={deck.only === "sessions"} onPress={() => toggleOnly("sessions")}>
            <SquareTerminal size={16} aria-hidden />
          </Pressed>
          <Pressed label="Needs me" pressed={deck.needs} onPress={toggleNeeds}>
            <Megaphone size={16} aria-hidden />
          </Pressed>
        </span>
        <Pressed label="Checks" pressed={deck.sel?.kind === "checks"} onPress={() => select(deck.sel?.kind === "checks" ? undefined : { kind: "checks", key: "checks" })}>
          <ShieldCheck size={16} aria-hidden />
        </Pressed>
        <span className="deck-controls__group" role="group" aria-label="View">
          <Pressed label="Map" keys="m" pressed={deck.view === "map"} onPress={() => setView("map")}>
            <Orbit size={16} aria-hidden />
          </Pressed>
          <Pressed label="Timeline" keys="t" pressed={deck.view === "timeline"} onPress={() => setView("timeline")}>
            <ChartNoAxesGantt size={16} aria-hidden />
          </Pressed>
        </span>
      </div>
    </section>
  );
}

function Incoming({ calls, now }: { calls: readonly Item[]; now: number }) {
  return (
    <section className="deck-panel deck-incoming" aria-label="Incoming calls">
      <header className="deck-band">
        Incoming calls
        <Tooltip label="Open the next call">
          <Kbd>w</Kbd>
        </Tooltip>
      </header>
      <ul className="deck-incoming__cards">
        {calls.slice(0, 3).map((item, at) => {
          const Icon = item.icon;
          return (
            <li key={item.key}>
              <button
                type="button"
                className="deck-hail"
                data-hue={item.hue}
                data-first={at === 0 || undefined}
                data-arrived={arrivedLate(item.key) || undefined}
                aria-label={`${item.kind}: ${item.title}`}
                onClick={() => {
                  visit(item.key);
                  select({ kind: "call", key: item.key });
                }}
              >
                <Tooltip label={item.kind}>
                  <span className="deck-hail__kind" role="img" aria-label={item.kind}>
                    <Icon size={16} aria-hidden />
                  </span>
                </Tooltip>
                <span className="deck-hail__title">{item.title}</span>
                <span className="deck-hail__age">{age(item.at, now)}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

// ---- The body ------------------------------------------------------------------------------------------

export function DeckBody(props: CockpitProps) {
  const ship = useShip(props);
  const deck = useDeck();
  const acts = useShipActs(props);
  const autopilot = useAutopilotLane({ picked: props.picked, state: props.state, now: props.now });
  noteCalls(ship.calls);
  // The first frame is over: a call that turns up from here on has arrived, and hails.
  useEffect(() => settle(), []);
  const listed = useMemo(() => listOf(ship, deck), [ship, deck.only, deck.needs, deck.held]);
  const incoming = incomingOf(ship.calls, deck.held, deck.visited);
  const selected = deck.sel?.key;
  return (
    <div className="armada-cockpit proto deck" data-direction="flightdeck">
      {autopilot.bar}
      <div className="deck-grid">
        <div className="deck-left">
          <List subjects={listed} cursor={deck.cursor} selected={deck.sel?.kind === "job" || deck.sel?.kind === "session" ? selected : undefined} onPick={(one) => select(selOfSubject(one))} />
          <Usage />
        </div>
        <div className="deck-centre">
          <Stage props={props} ship={ship} listed={listed} acts={acts} />
          <MergeQueue props={props} ship={ship} selected={deck.sel?.kind === "pr" ? selected : undefined} />
        </div>
        <Bays props={props} selected={deck.sel?.kind === "slot" ? selected : undefined} />
        <div className="deck-foot">
          <Controls props={props} autopilot={autopilot} />
          <Incoming calls={incoming} now={props.now} />
        </div>
      </div>
    </div>
  );
}

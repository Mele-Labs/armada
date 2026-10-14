// The Cockpit as a flight deck. A top bar (the real title bar, with the places as tabs in its middle), a left
// column of every Job and Session with the plan's usage under it, a dynamic area that is the map until something
// is picked and then that thing (a Job's canvas, a Session's thread, a bay, a pull request, a call), the merge
// queue as one dot a pull request, the bays on the right, and along the foot the flight controls and the calls
// that are incoming. Esc returns to the map or timeline exactly as it was left. The state is held in
// `deck-model.ts`, so the top bar, the body and the keys are one thing. Mock only. proto/cockpit-flight-deck.

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Box, ChartNoAxesGantt, GitMerge, Megaphone, Orbit, Pause, Plane, Power, ShieldCheck, SquareTerminal } from "lucide-react";
import { Badge, DropdownMenu, Kbd, Switch, Tabs, Tooltip } from "@armada/components";
import { stateOf as bayStateOf } from "@armada/components/src/compositions/PoolSlots/PoolTile";
import type { WorktreeSlot } from "@armada/protocol";
import { FIND_A_REPOSITORY, SURFACE, repositoryEntries, rootOf, useRecentRepositories } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";

import { age, type Item } from "../../Dashboard";
import type { AppSeamProps } from "../../cockpit/app-seam";
import { useAutopilotLane } from "../../cockpit/AutopilotLane";
import type { CockpitProps } from "../../cockpit/body-seam";
import { holdFor } from "../../cockpit/Cockpit";
import { dotsOf } from "../../cockpit/horizon";
import { useFleetEvents } from "../../cockpit/fleet-events";
import { MOST_PLAYING, droneOf, droneOfEvent, playOfEvent, playsOf, trailsOf, type Play } from "./deck-field";
import { ActivityFeed, Timeline } from "../../cockpit/Timeline";
import { viewsOf } from "../../merge-line";
import { useSessions } from "../../sessions-draft";
import { CallPane, ChecksPane, JobPane, PaneHead, PrPane, SessionPane, SlotPane } from "./deck-panes";
import {
  arrivedLate,
  callTrail,
  callOfSubject,
  deckNow,
  enterAutopilot,
  incomingOf,
  leaveAutopilot,
  listOf,
  moveCursor,
  noteCalls,
  openCall,
  release,
  scene,
  select,
  selOfPr,
  selOfSubject,
  setFocus,
  setHere,
  startReplay,
  stopReplay,
  setView,
  settle,
  slotKey,
  toggleChecks,
  toggleNeeds,
  toggleOnly,
  useDeck,
} from "./deck-model";
import { MarkGlyph, useShipActs } from "./shared";
import { useShip, type Pr, type Ship, type Subject } from "./ship";
import { toneOf } from "./tabs-model";
import { BARE, TREES, touchesOf } from "./deck-paths";
import { Battlefield, type DroneFacts, type GoalIn, type PadIn, type PrIn } from "./Battlefield";
import { DeckKeySheet, PANELS, PanelKey, armChord, cycleFrom, focusPanel, panelAt, remember, stepBy, stepIn, useDeckKeys, type PanelId } from "./deck-keys";
import { doingOfEvent, withDoing } from "./deck-doing";
import { claimsReplay, fieldAt, inOrder, useReplayDriver, useReplayKeys } from "./deck-replay";
import { FocusChip, ReplayBar, ReplayMark } from "./deck-replay-bar";
import "./deck.css";
import "./battlefield.css";

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

const SEARCH_TIP = "Search, or describe work to dispatch (⌘K)";

/** The real title bar's two ends, found once it is drawn. The tabs and the wordmark are portalled into them. */
function useTitleBar(): { start: HTMLElement | undefined; centre: HTMLElement | undefined } {
  const [hosts, setHosts] = useState<{ start: HTMLElement | undefined; centre: HTMLElement | undefined }>({ start: undefined, centre: undefined });
  useLayoutEffect(() => {
    const find = () => {
      const start = document.querySelector<HTMLElement>(".armada-title-bar__start") ?? undefined;
      const centre = document.querySelector<HTMLElement>(".armada-title-bar__center") ?? undefined;
      // The search trigger is the bar's own button, its label hidden by the deck's stylesheet: the tooltip names it and its key.
      const search = document.querySelector<HTMLElement>(".armada-title-bar__search");
      if (search !== null && search.title !== SEARCH_TIP) {
        search.title = SEARCH_TIP;
        search.setAttribute("aria-label", SEARCH_TIP);
      }
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
    // On a page opened from the deck the Cockpit tab is already the place: it goes back, to the deck as it was left.
    if (id === "cockpit" && place === "cockpit" && !here) return props.back();
    if (id === place) return;
    if (id === "cockpit") props.goTo(SURFACE.overview);
    else if (id === "jobs") props.goTo(SURFACE.jobs);
    else if (id === "sessions") props.goTo(SURFACE.sessions);
    else props.goTo(SURFACE.kit);
  };

  const incoming = incomingOf(ship.calls, deck.held, deck.visited);
  const listed = useMemo(() => listOf(ship, deck), [ship, deck.only, deck.needs, deck.checks, deck.held]);
  // `w` goes on to the next call, or forward along the ones it has opened; Shift+W goes back along them. A call answered since is passed over.
  const walkCalls = (by: 1 | -1) => {
    const live = (key: string) => ship.calls.find((one) => one.key === key && !deckNow().held.has(key));
    // The opened ones in the direction asked, from the one on show; the first still carried is the one.
    const order = by === -1 ? callTrail.keys.map((_, at) => at).filter((at) => at < (callTrail.at < 0 ? callTrail.keys.length : callTrail.at)).reverse() : callTrail.keys.map((_, at) => at).filter((at) => callTrail.at >= 0 && at > callTrail.at);
    const at = order.find((one) => live(callTrail.keys[one]!) !== undefined);
    let to = at === undefined ? undefined : live(callTrail.keys[at]!);
    if (at !== undefined) callTrail.at = at;
    else if (by === 1 && incoming[0] !== undefined) {
      to = incoming[0];
      callTrail.keys.push(to.key);
      callTrail.at = callTrail.keys.length - 1;
    }
    if (to === undefined) return;
    openCall(ship, to);
    if (!deckNow().here) props.goTo(SURFACE.overview);
  };
  const [keySheet, setKeySheet] = useState(false);
  useEffect(() => {
    const noted = (event: FocusEvent) => remember(event.target);
    document.addEventListener("focusin", noted);
    return () => document.removeEventListener("focusin", noted);
  }, []);
  /** Focus goes where the key said once the area has drawn what it was asked to. */
  const focusSoon = (id: PanelId) => window.requestAnimationFrame(() => focusPanel(id, deckNow().cursor));
  /** Pick a row of the list: the cursor moves with focus, and the row is brought into view. */
  const rowFocus = (key: string) => {
    moveCursor(key);
    // At once where the row is drawn, so a held key does not step from a cursor the last press has not reached; after a frame where it is not yet.
    const row = () => document.querySelector<HTMLElement>(`[data-row="${CSS.escape(key)}"]`);
    if (row() !== null) row()!.focus();
    else window.requestAnimationFrame(() => row()?.focus());
  };

  useDeckKeys((event) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const state = deckNow();
    const target = event.target as HTMLElement;
    if (holdsText(event.target) && target.getAttribute("role") !== "switch") {
      // The first Esc leaves the box, the next one acts.
      if (event.key === "Escape" && state.here && state.sel?.kind !== "call" && target.closest(".deck") !== null) {
        event.preventDefault();
        target.blur();
      }
      return;
    }
    if (target.closest?.('[role="menu"]') != null) return;
    if (event.key === "w" || event.key === "W") {
      event.preventDefault();
      return walkCalls(event.key === "w" ? 1 : -1);
    }
    // Two keys: the page in front reads neither (`armChord` takes the second before it can). Not over a Job's page, where `g` is its own.
    if (event.key === "g" && props.openJobId === null) {
      event.preventDefault();
      return armChord({ c: () => go("cockpit"), j: () => go("jobs"), s: () => go("sessions"), y: () => go("system") });
    }
    if (!state.here) return;
    if (event.key === "?") {
      event.preventDefault();
      return setKeySheet(true);
    }
    if (state.sel?.kind === "call") return;
    const panel = panelAt(target);
    // A replay's own keys, taken by it where it is up.
    if (state.replay !== undefined && claimsReplay(event, panel?.id)) return;
    if (event.key === "Tab" || event.key === "F6") {
      // Tab goes round the panels from any of them but the area while it holds a page of its own, whose controls it is left to reach; F6 always.
      if (event.key === "Tab" && (panel === undefined || (panel.id === "map" && state.sel !== undefined))) return;
      event.preventDefault();
      return focusPanel(cycleFrom(panel?.id, event.shiftKey ? -1 : 1), state.cursor);
    }
    if (/^[1-6]$/.test(event.key) && !event.shiftKey) {
      event.preventDefault();
      return TILES[Number(event.key) - 1]!.press();
    }
    // In a Job's pane, 0 is the canvas's own Fit: the whole run in the frame, from the step at work it opens on.
    if (event.key === "0" && !event.shiftKey && state.sel?.kind === "job") {
      const fit = document.querySelector<HTMLElement>('.deck-pane[aria-label^="Job:"] .deck-job__canvas button[aria-label="Fit"]');
      if (fit !== null) {
        event.preventDefault();
        return fit.click();
      }
    }
    // Zoom is the map's wherever focus is, while the map is what is showing.
    if (["+", "=", "-", "_", "0", "Home"].includes(event.key) && state.sel === undefined && state.view === "map") {
      const host = document.querySelector(".bf");
      if (host !== null && !host.contains(target)) {
        event.preventDefault();
        host.dispatchEvent(new KeyboardEvent("keydown", { key: event.key, bubbles: true, cancelable: true }));
      }
      return;
    }
    const goes = event.shiftKey ? undefined : PANELS.find((one) => one.key === event.key && one.id !== "map");
    if (goes !== undefined) {
      event.preventDefault();
      return focusPanel(goes.id, state.cursor);
    }
    switch (event.key) {
      case "ArrowDown":
      case "ArrowUp":
      case "ArrowLeft":
      case "ArrowRight":
      case "j":
      case "k": {
        // j and k are down and up, and next and previous along a row. The map's arrows pan, and a page in the area has its own. Outside the deck, they are not ours.
        if (panel?.id === "map" || (panel === undefined && target !== document.body && target.closest(".deck") === null)) return;
        // With nothing focused, left and right are not ours: a walk's, if one is playing.
        if (panel === undefined && (event.key === "ArrowLeft" || event.key === "ArrowRight")) return;
        event.preventDefault();
        const letter = event.key === "j" || event.key === "k";
        const down = event.key === "ArrowDown" || event.key === "j";
        if (panel !== undefined && panel.id !== "list") return letter ? stepBy(panel, target, down ? 1 : -1) : stepIn(panel, target, event.key);
        if (event.key === "ArrowLeft" || event.key === "ArrowRight") return;
        const at = listed.findIndex((one) => one.key === state.cursor);
        const next = listed[Math.min(listed.length - 1, Math.max(0, at < 0 ? (down ? 0 : listed.length - 1) : at + (down ? 1 : -1)))];
        if (next !== undefined) rowFocus(next.key);
        return;
      }
      case "Enter": {
        // Pressed on a button of the deck: the press, taken here so a walk playing does not also step on it.
        if (target.tagName === "BUTTON" || target.tagName === "A") {
          if (target.closest(".deck") === null) return;
          event.preventDefault();
          return target.click();
        }
        // A thing on the map has its own Enter.
        if (panel?.id === "map") return;
        if (panel !== undefined && panel.id !== "list") {
          // Pressed on the panel itself rather than on a thing in it: the thing focus is on.
          const item = document.activeElement instanceof HTMLElement && panel.items !== undefined ? document.activeElement.closest<HTMLElement>(panel.items) : null;
          if (item === null) return;
          event.preventDefault();
          return item.click();
        }
        const row = listed.find((one) => one.key === state.cursor);
        if (row === undefined) return;
        event.preventDefault();
        select(selOfSubject(row));
        return;
      }
      case "Escape":
        if (state.sel?.kind === "autopilot") {
          event.preventDefault();
          leaveAutopilot();
        } else if (state.sel !== undefined) {
          event.preventDefault();
          select(undefined);
          // What had focus may have been what was just put away: the list has it then.
          window.requestAnimationFrame(() => panelAt(document.activeElement) === undefined && focusPanel("list", deckNow().cursor));
        } else if (state.focus !== undefined) {
          event.preventDefault();
          setFocus(undefined);
        } else if ((panel !== undefined && panel.id !== "list") || target === document.body) {
          event.preventDefault();
          focusPanel("list", state.cursor);
        }
        return;
      case "m":
        event.preventDefault();
        setView("map");
        return focusSoon("map");
      case "t":
        event.preventDefault();
        setView("timeline");
        return focusSoon("map");
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
      <DeckKeySheet open={keySheet} onClose={() => setKeySheet(false)} />
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
    <section className="deck-panel deck-list" aria-label="Jobs and Sessions" tabIndex={-1}>
      <header className="deck-band">
        Jobs and Sessions
        <PanelKey id="list" />
      </header>
      <ul className="deck-list__rows">
        {subjects.map((one) => {
          const Kind = one.kind === "job" ? Box : SquareTerminal;
          const quiet = one.since ?? one.step?.label ?? "";
          const said = quiet.length > 9 ? "" : quiet;
          return (
            <li key={one.key}>
              <button type="button" className="deck-row" data-row={one.key} data-tone={toneOf(one)} data-selected={selected === one.key || undefined} data-cursor={cursor === one.key || undefined} aria-label={one.title} aria-current={selected === one.key || undefined} onClick={() => onPick(one)} onFocus={() => moveCursor(one.key)}>
                <Tooltip label={one.kind === "job" ? "Job" : "Session"}>
                  <span className="deck-row__kind" role="img" aria-label={one.kind === "job" ? "Job" : "Session"}>
                    <Kind size={14} aria-hidden />
                  </span>
                </Tooltip>
                <span className="deck-row__title">{one.title}</span>
                <span className="deck-row__said">{said}</span>
                <MarkGlyph mark={{ ...one.mark, tone: toneOf(one) }} />
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** What Fleet has been doing, newest first, in the column under the list. A press picks the Job or Session as a list row does. */
function Activity({ props, listed, ship }: { props: CockpitProps; listed: readonly Subject[]; ship: Ship }) {
  const deck = useDeck();
  const views = useMemo(() => viewsOf(props.state), [props.state]);
  const shown = useMemo(() => new Set(listed.map((one) => one.key)), [listed]);
  const keep = useMemo(() => (subject: { kind: string; id?: string }) => (subject.kind === "main" ? deck.only === undefined && !deck.needs && !deck.checks : shown.has(`${subject.kind}:${subject.id}`)), [shown, deck.only, deck.needs, deck.checks]);
  const pick = (key: string) => {
    const subject = ship.subjects.find((one) => one.key === key);
    if (subject !== undefined && subject.kind !== "pr") select(selOfSubject(subject));
  };
  // The row held lit: the one picked, or the one the playhead has last passed. It is brought into view.
  const events = useFleetEvents();
  const lit = useMemo(() => {
    if (deck.replay === undefined) return deck.focus;
    const { at } = deck.replay;
    return inOrder(events).filter((one) => Date.parse(one.at) <= at).at(-1)?.id.toString();
  }, [events, deck.focus, deck.replay === undefined ? undefined : deck.replay.at]);
  useEffect(() => {
    if (lit !== undefined) document.querySelector(`.deck-activity [data-event="${CSS.escape(lit)}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [lit]);
  return (
    <section className="deck-panel deck-activity" aria-label="Activity" tabIndex={-1}>
      <header className="deck-band">Activity</header>
      <ActivityFeed
        stacked
        now={props.now}
        jobs={props.state.jobs}
        views={views}
        filter="running"
        keep={keep as never}
        onOpenJob={(id) => pick(`job:${id}`)}
        onOpenSession={(id) => pick(`session:${id}`)}
        onPick={(event) => setFocus(event.id, event.subject.kind === "main" ? undefined : `${event.subject.kind}:${event.subject.id}`)}
        {...(lit === undefined ? {} : { lit })}
      />
    </section>
  );
}

// ---- The dynamic area ---------------------------------------------------------------------------------

/** What the field plays, from the events and from what changes while it is drawn: each played once, and the most that move at once is capped. */
function usePlays(candidates: readonly Play[], drones: readonly string[], landed: ReadonlyMap<number, string>): Play[] {
  const [plays, setPlays] = useState<Play[]>(() => candidates.filter((one) => !scene.seen.has(one.id)));
  const shown = useRef(plays);
  shown.current = plays;
  // What is shown now is shown: it is not played again when the field is drawn another time.
  useEffect(() => {
    for (const one of plays) scene.seen.add(one.id);
    if (plays.length === 0) return;
    const timer = window.setTimeout(() => setPlays([]), 8000);
    return () => window.clearTimeout(timer);
  }, [plays]);
  // Arrivals: a drone that was not there, a pull request that has landed, an event that was not there.
  useEffect(() => {
    const was = scene.drones;
    const landedWas = scene.landed;
    scene.drones = new Set(drones);
    scene.landed = new Set([...landed.keys()].map(String));
    const arrived: Play[] = [];
    if (was !== undefined) for (const key of drones) if (!was.has(key)) arrived.push({ id: `arrived:${key}`, kind: "launch", key, at: Date.now(), delay: 0 });
    if (landedWas !== undefined) for (const [number, root] of landed) if (!landedWas.has(String(number)) && !scene.seen.has(`merged:${number}`)) arrived.push({ id: `merged:${number}`, kind: "capture", root, number, at: Date.now(), delay: 0 });
    const late = [...candidates.filter((one) => !scene.seen.has(one.id) && !shown.current.some((p) => p.id === one.id)), ...arrived].filter((one) => !scene.seen.has(one.id));
    if (late.length > 0) setPlays((now) => [...now, ...late].slice(-MOST_PLAYING));
  }, [candidates, drones, landed]);
  return plays;
}

/** Everything the field draws, read off the ship, the events and the replay: the map shows all of it, a Job's mission the part that is its own. */
function useField(props: CockpitProps, ship: Ship, listed: readonly Subject[]) {
  const deck = useDeck();
  const sessions = useSessions();
  const events = useFleetEvents();
  const shown = useMemo(() => new Set(listed.map((one) => one.key)), [listed]);
  const slots = slotsOf(props);
  const repositories = props.state.holds.repositories ?? [];
  const rootOfManifest = (manifest: string | undefined) => repositories.find((one) => one.manifest?.id === manifest)?.root;

  const drones = useMemo(() => {
    const all = ship.subjects.filter((one) => one.kind !== "pr");
    const has = new Set(all.map((one) => one.key));
    const holder = new Map(
      slots.flatMap((slot) => {
        const held = slot.held;
        const key = held.state === "job" ? `job:${held.job_id}` : held.state === "hosted_session" ? `session:${held.session_id}` : undefined;
        return key === undefined ? [] : [[key, slotKey(slot)] as const];
      }),
    );
    return all.map((one) => {
      const attached = one.kind === "session" ? sessions.find((row) => row.id === one.id)?.attachments.find((a) => a.kind === "job" && a.looking !== true) : undefined;
      const made = one.kind === "job" ? props.state.jobs.find((job) => job.id === one.id)?.dispatched_by : undefined;
      const parent = attached !== undefined && attached.kind === "job" ? `job:${attached.id}` : made === undefined ? undefined : `job:${made}`;
      return droneOf(one, callOfSubject(one, deck.held) !== undefined, { parent: parent !== undefined && has.has(parent) ? parent : undefined, slot: holder.get(one.key), shown: shown.has(one.key), touches: touchesOf(one.title) });
    });
  }, [ship.subjects, sessions, slots, props.state.jobs, deck.held, shown]);

  const repos = useMemo(() => {
    const roots = [...new Set([...ship.subjects.filter((one) => one.kind !== "pr").map((one) => one.root), ...ship.mains.map((one) => one.root), ...slots.map((one) => rootOfManifest(one.manifest_id) ?? one.manifest_id)])].sort();
    return roots.map((root) => ({
      root,
      name: ship.repos.find((one) => one.root === root)?.name ?? ship.subjects.find((one) => one.root === root)?.repo ?? root,
      pads: slots.filter((one) => (rootOfManifest(one.manifest_id) ?? one.manifest_id) === root).sort((a, b) => a.slot - b.slot).map(slotKey),
      goal: ship.mains.some((one) => one.root === root),
      tree: TREES[ship.repos.find((one) => one.root === root)?.name ?? ship.subjects.find((one) => one.root === root)?.repo ?? root] ?? [],
      bare: BARE,
    }));
  }, [ship.subjects, ship.mains, ship.repos, slots, repositories]);

  const pads = useMemo(() => {
    const out = new Map<string, PadIn>();
    for (const slot of slots) {
      const state = bayStateOf({ slot });
      const held = slot.held;
      const holder = held.state === "job" ? (held.job_title ?? held.job_id) : held.state === "hosted_session" ? (held.session_title ?? held.session_id) : undefined;
      out.set(slotKey(slot), { key: slotKey(slot), slot: slot.slot, bay: slot.closed === true ? "closed" : state.bay, word: slot.closed === true ? "Closed" : state.word, repo: repoNameOf(props, slot.manifest_id), Glyph: state.Glyph, ...(holder === undefined ? {} : { holder }) });
    }
    return out;
  }, [slots]);

  const goals = useMemo(() => new Map<string, GoalIn>(ship.mains.map((main) => [main.root, { root: main.root, red: main.state === "red", said: main.state === "red" ? `Main is red in ${main.repo}` : `Main is green in ${main.repo}` }])), [ship.mains]);

  const prs = useMemo<PrIn[]>(() => ship.prs.map((pr) => ({ key: pr.key, number: pr.number, stage: pr.stage, root: pr.root, ...(pr.owner === undefined ? {} : { owner: `${pr.owner.kind}:${pr.owner.id}` }) })), [ship.prs]);
  const landed = useMemo(() => new Map(ship.prs.filter((pr) => pr.stage === "landed").map((pr) => [pr.number, pr.root] as const)), [ship.prs]);
  const ownerOf = (number: number) => {
    const pr = ship.prs.find((one) => one.number === number);
    return pr?.owner === undefined ? undefined : `${pr.owner.kind}:${pr.owner.id}`;
  };
  const candidates = useMemo(() => playsOf(events, props.now, ownerOf, rootOfManifest), [events, props.now, ship.prs, repositories]);
  const trails = useMemo(() => trailsOf(events, props.now, ownerOf), [events, props.now, ship.prs]);
  const plays = usePlays(candidates, useMemo(() => drones.map((one) => one.key), [drones]), landed);

  // A picked event, and a replay: the field as it was at the playhead, from the events up to it.
  const ordered = useMemo(() => inOrder(events), [events]);
  const replay = deck.replay;
  const passed = replay === undefined ? 0 : ordered.filter((one) => Date.parse(one.at) <= replay.at).length;
  const before = useMemo(() => (replay === undefined ? ordered : ordered.filter((one) => Date.parse(one.at) <= replay.at)), [ordered, replay === undefined, passed]);
  const field = useMemo(() => (replay === undefined ? undefined : fieldAt(ordered, replay.at, drones, prs, ownerOf)), [ordered, replay === undefined, replay?.run, passed, drones, prs]);
  const picked = deck.focus === undefined ? undefined : ordered.find((one) => String(one.id) === deck.focus);
  const focusKey = picked === undefined ? undefined : droneOfEvent(picked, ownerOf);
  // What each working drone is doing, from the tool events up to the playhead; a picked tool event shows its own action on its own drone.
  const held = replay === undefined && picked !== undefined && focusKey !== undefined && doingOfEvent(picked) !== undefined ? { key: focusKey, event: picked } : undefined;
  const flying = useMemo(() => withDoing(field?.drones ?? drones, before, Infinity, ownerOf, held), [field, drones, before, held?.event.id]);
  const crates = field?.prs ?? prs;
  const flightTrails = useMemo(() => (replay === undefined ? trails : trailsOf(before, replay.at, ownerOf)), [trails, replay === undefined, passed]);
  // The event's own effect, once, where it is picked.
  const focusPlays = useMemo(() => {
    const play = picked === undefined ? undefined : playOfEvent(picked, `focus:${picked.id}`, ownerOf, rootOfManifest);
    return play === undefined ? [] : [play];
  }, [deck.focus]);
  const drawn = replay === undefined ? [...plays, ...focusPlays] : replay.plays;
  const clock = replay?.at ?? props.now;
  // What each drone's callout says besides the drone: its state as the list draws it, the call it holds, its last event, the hour of events as bars.
  const facts = useMemo(() => {
    const out = new Map<string, DroneFacts>();
    const HOUR = 60 * 60_000;
    const BUCKETS = 12;
    for (const drone of flying) {
      const subject = ship.subjects.find((one) => one.key === drone.key);
      if (subject === undefined) continue;
      const mine = before.filter((event) => droneOfEvent(event, ownerOf) === drone.key && Date.parse(event.at) <= clock + 60_000);
      const latest = mine.reduce<(typeof mine)[number] | undefined>((best, event) => (best === undefined || Date.parse(event.at) > Date.parse(best.at) ? event : best), undefined);
      const ticks = Array.from({ length: BUCKETS }, () => 0);
      for (const event of mine) {
        const ago = clock - Date.parse(event.at);
        if (ago >= 0 && ago < HOUR) ticks[BUCKETS - 1 - Math.floor(ago / (HOUR / BUCKETS))]! += 1;
      }
      const owed = callOfSubject(subject, deck.held);
      const call = owed === undefined ? undefined : ship.calls.find((one) => one.key === owed)?.title;
      const pad = drone.slot === undefined ? undefined : pads.get(drone.slot);
      out.set(drone.key, {
        state: { Glyph: subject.mark.Glyph, said: subject.mark.said, tone: toneOf(subject) },
        ticks,
        ...(call === undefined ? {} : { call }),
        ...(latest === undefined ? {} : { last: { says: latest.says, ago: age(latest.at, clock) } }),
        ...(pad === undefined ? {} : { bay: { slot: pad.slot, Glyph: pad.Glyph, word: pad.word } }),
      });
    }
    return out;
  }, [flying, ship.subjects, ship.calls, before, replay === undefined ? props.now : passed, deck.held, pads, ship.prs]);

  return { deck, repos, pads, goals, crates, drones: flying, flightTrails, drawn, facts, focusKey, replay, ordered, picked, ownerOf, rootOfManifest };
}

function MapView({ props, ship, listed, cursor }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; cursor: string | undefined }) {
  const { replay, ordered, picked, repos, drones: flying, pads, goals, crates, flightTrails, drawn, facts, focusKey } = useField(props, ship, listed);
  return (
    <section className="deck-pane" aria-label="Flight map">
      <header className="deck-band deck-pane__head">
        <Orbit size={16} aria-hidden />
        <h2 className="deck-pane__title">Map</h2>
        <PanelKey id="map" />
      </header>
      <div className="deck-map">
        {replay === undefined ? null : <ReplayMark />}
        {replay !== undefined || picked === undefined ? null : <FocusChip event={picked} now={props.now} onReplay={() => startReplay(Date.parse(picked.at))} />}
        {replay === undefined ? null : <ReplayBar events={ordered} replay={replay} end={props.now} onLive={stopReplay} />}
        <Battlefield
          repos={repos}
          drones={flying}
          pads={pads}
          goals={goals}
          prs={crates}
          trails={flightTrails}
          plays={drawn}
          facts={facts}
          focus={replay === undefined ? focusKey : undefined}
          selected={cursor}
          onSelect={(key) => {
            const subject = ship.subjects.find((one) => one.key === key);
            if (subject !== undefined) select(selOfSubject(subject));
          }}
          onPad={(key) => select({ kind: "slot", key })}
          onGoal={(root) => {
            const pr = ship.prs.find((one) => one.root === root && one.stage !== "landed") ?? ship.prs.find((one) => one.root === root);
            if (pr !== undefined) select(selOfPr(pr));
          }}
          onPr={(key) => {
            const pr = ship.prs.find((one) => one.key === key);
            if (pr !== undefined) select(selOfPr(pr));
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
    () => (subject: { kind: string; id?: string }) => (subject.kind === "main" ? deck.only === undefined && !deck.needs && !deck.checks : shown.has(`${subject.kind}:${subject.id}`)),
    [shown, deck.only, deck.needs, deck.checks],
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
        <Tooltip label="Timeline" shortcut="t">
          <Kbd>t</Kbd>
        </Tooltip>
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

function Stage({ props, ship, listed, acts, autopilot }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; acts: ReturnType<typeof useShipActs>; autopilot: ReturnType<typeof useAutopilotLane> }) {
  const deck = useDeck();
  const sel = deck.sel;
  const subject = sel === undefined ? undefined : ship.subjects.find((one) => one.key === sel.key && one.kind !== "pr");
  const slot = sel?.kind === "slot" ? slotsOf(props).find((one) => slotKey(one) === sel.key) : undefined;
  const pr = sel?.kind === "pr" ? ship.prs.find((one) => one.key === sel.key) : undefined;
  const call = sel?.kind === "call" ? ship.calls.find((one) => one.key === sel.key) : undefined;
  // What was picked is gone (a Job ended, a call was answered elsewhere): back to the map.
  const checks = sel?.kind === "checks";
  const lane = sel?.kind === "autopilot";
  const gone = sel !== undefined && !checks && !lane && subject === undefined && slot === undefined && pr === undefined && call === undefined;
  useEffect(() => {
    if (gone) select(undefined);
  }, [gone]);
  const onPr = (one: Pr) => select(selOfPr(one));
  const owed = subject === undefined ? undefined : callOfSubject(subject, deck.held);
  const asked = owed === undefined ? undefined : ship.calls.find((one) => one.key === owed);
  if (sel?.kind === "job" && subject !== undefined) return <JobPane subject={subject} props={props} acts={acts} onPr={onPr} call={asked} />;
  if (sel?.kind === "session" && subject !== undefined) return <SessionPane subject={subject} props={props} acts={acts} onPr={onPr} call={asked} />;
  if (slot !== undefined) return <SlotPane slot={slot} repo={repoNameOf(props, slot.manifest_id)} now={props.now} />;
  if (pr !== undefined) return <PrPane pr={pr} owner={ship.subjects.find((one) => one.pr?.key === pr.key)} props={props} acts={acts} />;
  if (call !== undefined) return <CallPane item={call} props={props} />;
  if (checks) return <ChecksPane props={props} ship={ship} />;
  if (lane) return <AutopilotPane props={props} autopilot={autopilot} />;
  return deck.view === "map" ? <MapView props={props} ship={ship} listed={listed} cursor={deck.cursor} /> : <TimelineView props={props} ship={ship} listed={listed} />;
}

/** The merge queue's two takes of one row, chosen by `?merge=`: a dotted belt, or small chips. Dots when it names neither. */
const mergeTake = (): "dots" | "chips" => (new URLSearchParams(window.location.search).get("merge") === "chips" ? "chips" : "dots");

/**
 * The merge queue: one row across the repositories, one mark for each pull request, each repository's nearest main
 * first, ending at one light for main, red if any is (the chips take puts main first, to the left). A press opens the pull request in the area. No count.
 */
function MergeQueue({ props, ship, selected }: { props: CockpitProps; ship: Ship; selected: string | undefined }) {
  const sessions = useSessions();
  const views = viewsOf(props.state);
  if (views.length === 0) return null;
  const take = mergeTake();
  const red = views.filter((view) => view.hub?.main?.state === "red").map((view) => ship.repos.find((one) => one.root === view.root)?.name ?? view.root);
  const mainSaid = red.length > 0 ? `Main is red in ${red.join(", ")}` : "Main is green";
  const marks = views.flatMap((view, at) => [
    ...(at === 0 ? [] : [<li key={`gap:${view.root}`} className="deck-queue__gap" aria-hidden="true" />]),
    ...dotsOf(view, props.state.mergeLines, views, sessions).map((dot) => {
      const pr = ship.prs.find((one) => one.root === view.root && `pull:${one.number}` === dot.key);
      const owner = pr === undefined ? undefined : ship.subjects.find((one) => one.pr?.key === pr.key);
      const tone = pr?.stage === "red" ? "bad" : owner?.mergeable === true ? "ask" : "calm";
      const lit = { "data-state": dot.state, "data-queued": dot.queued || undefined, "data-tone": tone };
      if (take === "chips") {
        const inside = (
          <>
            <span className="deck-chip__dot" {...lit} aria-hidden="true" />
            <span className="deck-chip__number">{dot.card.heading}</span>
          </>
        );
        return (
          <li key={`${view.root}:${dot.key}`}>
            <Tooltip label={dot.tip} asChild>
              {pr === undefined ? (
                <span className="deck-chip" role="img" aria-label={dot.tip}>
                  {inside}
                </span>
              ) : (
                <button type="button" className="deck-chip" data-selected={selected === pr.key || undefined} aria-label={dot.tip} onClick={() => select(selOfPr(pr))}>
                  {inside}
                </button>
              )}
            </Tooltip>
          </li>
        );
      }
      return (
        <li key={`${view.root}:${dot.key}`} className="armada-view__dot-item">
          <Tooltip label={dot.tip} asChild>
            {pr === undefined ? (
              <span className="armada-view__dot" {...lit} role="img" aria-label={dot.tip} />
            ) : (
              <button type="button" className="armada-view__dot" {...lit} data-selected={selected === pr.key || undefined} aria-label={dot.tip} onClick={() => select(selOfPr(pr))} />
            )}
          </Tooltip>
        </li>
      );
    }),
  ]);
  return (
    <section className={`deck-panel deck-queue${take === "dots" ? " armada-view__horizon" : ""}`} data-take={take} aria-label="Merge queue" tabIndex={-1}>
      {take === "chips" ? (
        <div className="deck-queue__row">
          <GitMerge size={16} aria-hidden="true" />
          <ol className="deck-chips">
            <li>
              <Tooltip label={mainSaid} asChild>
                <span className="deck-chip" data-main role="img" aria-label={mainSaid}>
                  <span className="deck-chip__dot" data-tone={red.length > 0 ? "bad" : "calm"} aria-hidden="true" />
                  <span className="deck-chip__number">main</span>
                </span>
              </Tooltip>
            </li>
            {marks}
          </ol>
          <PanelKey id="queue" />
        </div>
      ) : (
        <div className="armada-view__line">
          <GitMerge size={16} aria-hidden="true" />
          <ol className="armada-view__belt">{marks}</ol>
          <Tooltip label={mainSaid}>
            <span className="armada-view__main" data-red={red.length > 0 || undefined} />
          </Tooltip>
          <PanelKey id="queue" />
        </div>
      )}
    </section>
  );
}

// ---- The right column, and the foot ------------------------------------------------------------------

function Bays({ props, selected }: { props: CockpitProps; selected: string | undefined }) {
  const slots = slotsOf(props);
  return (
    <section className="deck-panel deck-bays" aria-label="Bays" tabIndex={-1}>
      <header className="deck-band">
        Bays
        <PanelKey id="bays" />
      </header>
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

type Deck = ReturnType<typeof deckNow>;
/** The six tiles in the order their keys, 1 to 6, press them. */
const TILES: readonly { label: string; tip?: string; text: string; Glyph: typeof Box; also?: string; pressed: (deck: Deck) => boolean; press: () => void }[] = [
  { label: "Jobs only", text: "Jobs", Glyph: Box, pressed: (deck) => deck.only === "jobs", press: () => toggleOnly("jobs") },
  { label: "Sessions only", text: "Sessions", Glyph: SquareTerminal, pressed: (deck) => deck.only === "sessions", press: () => toggleOnly("sessions") },
  { label: "Checks", tip: "Checks: only what is running one", text: "Checks", Glyph: ShieldCheck, pressed: (deck) => deck.checks, press: toggleChecks },
  { label: "Needs me", text: "Calls", Glyph: Megaphone, pressed: (deck) => deck.needs, press: toggleNeeds },
  { label: "Map", text: "Map", Glyph: Orbit, also: "m", pressed: (deck) => deck.view === "map", press: () => setView("map") },
  { label: "Timeline", text: "Timeline", Glyph: ChartNoAxesGantt, also: "t", pressed: (deck) => deck.view === "timeline", press: () => setView("timeline") },
];

/** The four filters and the view switch: one row of six equal tiles. */
function FlightControl() {
  const deck = useDeck();
  return (
    <section className="deck-panel deck-controls" aria-label="Flight controls" tabIndex={-1}>
      <header className="deck-band">
        Flight control
        <PanelKey id="controls" />
      </header>
      <div className="deck-controls__block">
        {TILES.map((tile, at) => (
          <Tooltip key={tile.label} label={tile.tip ?? tile.label} shortcut={tile.also === undefined ? String(at + 1) : `${at + 1} / ${tile.also}`} asChild>
            <button type="button" className="deck-tile" aria-label={tile.label} aria-pressed={tile.pressed(deck)} onClick={tile.press}>
              <tile.Glyph size={20} aria-hidden />
              <span className="deck-tile__text">{tile.text}</span>
              <Kbd className="deck-tile__key" aria-hidden="true">
                {at + 1}
              </Kbd>
            </button>
          </Tooltip>
        ))}
      </div>
    </section>
  );
}

const STATE_BADGE = {
  off: { status: "not-started", icon: Power, said: "Off" },
  running: { status: "running", icon: Plane, said: "Running" },
  paused: { status: "awaiting-review", icon: Pause, said: "Waiting" },
} as const;

/** The repository's picker, where none is chosen. Choosing one opens the Autopilot lane for it. */
function RepositoryPicker({ props, autopilot }: { props: CockpitProps; autopilot: ReturnType<typeof useAutopilotLane> }) {
  const [recent, remember] = useRecentRepositories();
  return (
    <DropdownMenu
      align="end"
      portal
      icon={Plane}
      triggerLabel="Repository"
      searchLabel={FIND_A_REPOSITORY}
      entries={repositoryEntries(props.state.holds.repositories ?? [], { onlySetUp: true, recent })}
      onSelect={(chosen) => {
        const root = rootOf(chosen);
        remember(root);
        autopilot.control?.openNext();
        props.onPickRepository?.(root);
      }}
    />
  );
}

/**
 * Autopilot as a compact card beside the controls: the switch is its title, and under it the repository, how it
 * stands and a thin bar. Switching it on gives the area to its configuration; off, the area is as it was.
 */
function AutopilotPanel({ props, autopilot }: { props: CockpitProps; autopilot: ReturnType<typeof useAutopilotLane> }) {
  const deck = useDeck();
  const control = autopilot.control;
  const open = deck.sel?.kind === "autopilot";
  const state = control?.state ?? "off";
  const badge = STATE_BADGE[state];
  const flip = () => {
    if (!open) return enterAutopilot();
    leaveAutopilot();
    if (control?.open === true) control.toggle();
  };
  return (
    <section className="deck-panel deck-autopilot" aria-label="Autopilot" tabIndex={-1}>
      <header className="deck-band">
        <Switch checked={open || state !== "off"} onChange={flip} disabled={control === undefined}>
          Autopilot
        </Switch>
        <PanelKey id="autopilot" />
      </header>
      <div className="deck-autopilot__body">
        <div className="deck-autopilot__repo">
          {props.picked === null ? (
            <RepositoryPicker props={props} autopilot={autopilot} />
          ) : (
            <span className="deck-autopilot__name">{props.picked.manifest?.repository ?? props.picked.root}</span>
          )}
          <Badge status={badge.status} icon={badge.icon}>
            {badge.said}
          </Badge>
        </div>
        <div className="deck-autopilot__bar" data-state={state} role="progressbar" aria-label={`Autopilot, ${badge.said.toLowerCase()}`}>
          <span className="deck-autopilot__fill" />
        </div>
      </div>
    </section>
  );
}

/** The lane in the dynamic area: the real bar for the chosen repository, or the picker while none is. */
function AutopilotPane({ props, autopilot }: { props: CockpitProps; autopilot: ReturnType<typeof useAutopilotLane> }) {
  // The bar is the lane's own, drawn while its control has it open; opening this pane is what opens it.
  const control = autopilot.control;
  useEffect(() => {
    if (props.picked !== null && control !== undefined && !control.open) control.openNext();
  }, [props.picked, control]);
  return (
    <section className="deck-pane" aria-label="Autopilot lane">
      <PaneHead glyph={Plane} kind="Autopilot" title="Autopilot" />
      <div className="deck-pane__body">{props.picked === null ? <RepositoryPicker props={props} autopilot={autopilot} /> : autopilot.bar}</div>
    </section>
  );
}

function Incoming({ calls, now, ship }: { calls: readonly Item[]; now: number; ship: Ship }) {
  return (
    <section className="deck-panel deck-incoming" aria-label="Incoming calls" tabIndex={-1}>
      <header className="deck-band">
        Incoming calls
        <PanelKey id="calls" />
        <Tooltip label="Open the next call" shortcut="w">
          <Kbd>w</Kbd>
        </Tooltip>
      </header>
      <ul className="deck-incoming__cards">
        {calls.map((item, at) => {
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
                onClick={() => openCall(ship, item)}
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
  // A replay runs from here, not from the map: it carries on while another page of the area is open.
  const events = useFleetEvents();
  const repositories = props.state.holds.repositories ?? [];
  const ownerOf = (number: number) => {
    const owned = ship.prs.find((one) => one.number === number)?.owner;
    return owned === undefined ? undefined : `${owned.kind}:${owned.id}`;
  };
  const rootOf = (manifest: string | undefined) => repositories.find((one) => one.manifest?.id === manifest)?.root;
  useReplayDriver(events, props.now, ownerOf, rootOf);
  useReplayKeys(deck.replay !== undefined, events, ownerOf, rootOf, (target) => panelAt(target)?.id);
  // The first frame is over: a call that turns up from here on has arrived, and hails.
  useEffect(() => settle(), []);
  const listed = useMemo(() => listOf(ship, deck), [ship, deck.only, deck.needs, deck.checks, deck.held]);
  const incoming = incomingOf(ship.calls, deck.held, deck.visited);
  const selected = deck.sel?.key;
  return (
    <div className="armada-cockpit proto deck" data-direction="flightdeck">
      <div className="deck-grid">
        <div className="deck-left">
          <List subjects={listed} cursor={deck.cursor} selected={deck.sel?.kind === "job" || deck.sel?.kind === "session" ? selected : undefined} onPick={(one) => select(selOfSubject(one))} />
          <Activity props={props} listed={listed} ship={ship} />
        </div>
        <div className="deck-centre">
          <div className="deck-stage">
            <Stage props={props} ship={ship} listed={listed} acts={acts} autopilot={autopilot} />
            <MergeQueue props={props} ship={ship} selected={deck.sel?.kind === "pr" ? selected : undefined} />
          </div>
          <div className="deck-foot">
            <FlightControl />
            <AutopilotPanel props={props} autopilot={autopilot} />
          </div>
        </div>
        <div className="deck-right">
          <Bays props={props} selected={deck.sel?.kind === "slot" ? selected : undefined} />
          <Incoming calls={incoming} now={props.now} ship={ship} />
        </div>
      </div>
    </div>
  );
}

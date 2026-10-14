// Direction B as a navigation model, not a dashboard: the Cockpit holds most of what the Work group of the left
// rail was. Its wall is the places, Jobs, Sessions, Checks, the merge line and the bays, each a live condensed
// tile, gray while well, with Autopilot and Main as marks. A place in trouble grows to the large tile and
// draws its subjects as strips with their acts in place. Pressing a tile, or `g` and a key, takes the real
// page for that place as a layer; opening a Session or a Job from anywhere does the same. A bar over the
// layer carries a Back chip, the crumbs, and the roll call, and Back returns to the Cockpit exactly as it was
// left: its station, selection, scroll, Peek and Reply live in the window's store and not in the body.
// The tray and the call card are over every layer. Mock only. proto/cockpit-three-directions.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { ChevronLeft, ChevronRight, Maximize2 } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";
import { SURFACE } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { AppSeamProps } from "../../cockpit/app-seam";
import type { CockpitProps } from "../../cockpit/body-seam";
import { useSessions } from "../../sessions-draft";
import { useCalls } from "./calls";
import { Rail } from "./rail";
import { KEYS, KeySheet, STATION_GLYPH, StationMark, chord, scrolled, setCardOpen, useShipActs, useShipKeys, useShipUi } from "./shared";
import { stationsOf, useShip, type Ship, type Station, type StationId, type Vote } from "./ship";
import { Strip } from "./Strips";
import { BaysBody, EdgeTile, MainBody, subjectsOf } from "./Wall";
import "./app.css";

const RANK: Record<Vote, number> = { go: 0, off: 0, hold: 1, nogo: 2 };
/** Where a tie goes first. */
const PRIORITY: readonly StationId[] = ["main", "merge", "sessions", "jobs", "checks", "bays", "autopilot"];
/** The places the wall holds, in the order the Work group had them. */
const PLACES = ["jobs", "sessions", "checks", "merge", "bays"] as const;
const ROLL: readonly StationId[] = ["jobs", "sessions", "checks", "merge", "bays", "autopilot", "main"];

/** The page each place is, and the key after `g` that goes to it. Studios and Retros are layers with no station. */
const SURFACE_OF: Partial<Record<StationId, string>> = { jobs: SURFACE.jobs, sessions: SURFACE.sessions, checks: SURFACE.checks, merge: SURFACE.mergeLine, bays: SURFACE.worktrees };
const CHORD: Record<string, string> = { j: SURFACE.jobs, s: SURFACE.sessions, c: SURFACE.checks, m: SURFACE.mergeLine, b: SURFACE.worktrees, t: SURFACE.studios, r: SURFACE.lessons };
const NAME: Record<string, string> = {
  [SURFACE.jobs]: "Jobs",
  [SURFACE.sessions]: "Sessions",
  [SURFACE.checks]: "Checks",
  [SURFACE.mergeLine]: "Merge line",
  [SURFACE.worktrees]: "Bays",
  [SURFACE.studios]: "Studios",
  [SURFACE.lessons]: "Retros",
  [SURFACE.settings]: "Settings",
  [SURFACE.kit]: "Kit",
  [SURFACE.mods]: "Mods",
  [SURFACE.guides]: "Guides",
  [SURFACE.workflows]: "Workflows",
  [SURFACE.doctor]: "Doctor",
  [SURFACE.manifest]: "Manifest",
};

/** The stations as marks: pressing one takes the Cockpit to it, wherever the window is. */
function RollCall({ stations, onPress }: { stations: readonly Station[]; onPress: (id: StationId) => void }) {
  return (
    <span className="proto-band__marks" role="group" aria-label="Roll call">
      {ROLL.map((id) => {
        const station = stations.find((one) => one.id === id);
        return station === undefined ? null : (
          <button key={id} type="button" className="proto-roll" onClick={() => onPress(id)}>
            <StationMark station={station} />
          </button>
        );
      })}
    </span>
  );
}

/** A pane that is where it was scrolled to when the body comes back. */
function Scroller({ id, className, children }: { id: string; className: string; children: ReactNode }) {
  const pane = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (pane.current !== null) pane.current.scrollTop = scrolled.get(id);
  }, [id]);
  return (
    <div ref={pane} className={className} onScroll={(event) => scrolled.set(id, event.currentTarget.scrollTop)}>
      {children}
    </div>
  );
}

/** The Cockpit's body in the app model: the wall of places, and nothing else. */
export function AppBody(props: CockpitProps & { goTo?: (surface: string) => void }) {
  const ship = useShip(props, false);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const stations = stationsOf(ship);
  const [hover, setHover] = useState<string | undefined>();
  const troubled = stations
    .filter((one) => RANK[one.vote] > 0)
    .sort((a, b) => RANK[b.vote] - RANK[a.vote] || PRIORITY.indexOf(a.id) - PRIORITY.indexOf(b.id));
  // A station that gets worse takes the large tile, whatever was chosen.
  const was = useRef<Partial<Record<StationId, number>>>({});
  const signature = stations.map((one) => `${one.id}:${RANK[one.vote]}`).join(" ");
  useEffect(() => {
    const worse = stations.some((one) => was.current[one.id] !== undefined && RANK[one.vote] > was.current[one.id]!);
    if (worse) ui.setPin(undefined);
    was.current = Object.fromEntries(stations.map((one) => [one.id, RANK[one.vote]]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);
  const lead = stations.find((one) => one.id === ui.pin) ?? troubled[0];
  const order = lead === undefined ? [] : subjectsOf(lead.id, ship);
  useShipKeys({ ship, order, ui, acts, cardOpen: ui.cardOpen, openCall: () => undefined });
  const LeadGlyph = lead === undefined ? undefined : STATION_GLYPH[lead.id];
  const lit = hover ?? ui.selected;
  const link = { setHover };
  const open = (id: StationId) => {
    const surface = SURFACE_OF[id];
    if (surface !== undefined) openPlace(surface);
  };
  const openPlace = (surface: string) => placeHost.current?.(surface);
  const alarm = troubled.some((one) => one.vote === "nogo");
  const edge = PLACES.map((id) => stations.find((one) => one.id === id)!).filter((one) => one !== lead);
  return (
    <div className="armada-cockpit proto" data-direction="app">
      <section className="proto-view" aria-label="Cockpit" data-alarm={alarm || undefined} data-waiting={troubled.length > 0 || undefined}>
        <header className="proto-band" aria-label="Roll call">
          <RollCall stations={stations} onPress={(id) => ui.setPin(id)} />
          <span className="proto-band__keys">
            <Tooltip label="Go to a place">
              <span>
                <Kbd>g</Kbd> <Kbd>s</Kbd>
              </span>
            </Tooltip>
            <Tooltip label="Open the call that arrived">
              <Kbd>{KEYS.call}</Kbd>
            </Tooltip>
            <Tooltip label="Keys">
              <Kbd>?</Kbd>
            </Tooltip>
          </span>
        </header>
        <div className="proto-stage">
          <div className="proto-wall" data-calm={lead === undefined || undefined}>
            {lead === undefined || LeadGlyph === undefined ? null : (
              <section className="proto-lead" data-vote={lead.vote} aria-label={lead.name}>
                <header className="proto-lead__band">
                  <LeadGlyph size={16} aria-hidden />
                  <span>{lead.name}</span>
                  <StationMark station={lead} />
                  {SURFACE_OF[lead.id] === undefined ? null : (
                    <Tooltip label={`Open ${lead.name}`}>
                      <Button variant="ghost" size="sm" iconOnly aria-label={`Open ${lead.name}`} onClick={() => open(lead.id)}>
                        <Maximize2 size={14} aria-hidden />
                      </Button>
                    </Tooltip>
                  )}
                </header>
                <Scroller id={`lead:${lead.id}`} className="proto-lead__body">
                  {lead.id === "main" && ship.main !== undefined ? (
                    <MainBody main={ship.main} acts={acts} />
                  ) : lead.id === "bays" ? (
                    <BaysBody bays={ship.bays} />
                  ) : lead.id === "merge" ? (
                    <div className="proto-rail proto-rail--lead">
                      <Rail ship={ship} ui={ui} acts={acts} lit={lit} link={link} />
                    </div>
                  ) : order.length === 0 ? null : (
                    <ul className="proto-grid" aria-label={`${lead.name} subjects`}>
                      {order.map((one) => (
                        <Strip key={one.key} subject={one} selected={ui.selected === one.key} linked={lit === one.key} ui={ui} acts={acts} link={link} />
                      ))}
                    </ul>
                  )}
                </Scroller>
              </section>
            )}
            <div className="proto-edge" data-calm={lead === undefined || undefined}>
              {edge.map((one) => (
                <EdgeTile key={one.id} station={one} ship={ship} onPress={() => open(one.id)} />
              ))}
            </div>
          </div>
          <KeySheet ui={ui} more={[[["g", "j s c m b t r"], "Go to a place"]]} />
        </div>
      </section>
    </div>
  );
}

/** How the body asks the window for a page: set by the chrome, which is given `goTo`. */
const placeHost: { current: ((surface: string) => void) | undefined } = { current: undefined };

/** The layer bar: Back, the crumbs, and the roll call, over any page that is not the Cockpit. */
function LayerBar({ crumbs, stations, onBack, onPress, acts }: { crumbs: readonly { label: string; press: () => void }[]; stations: readonly Station[]; onBack: () => void; onPress: (id: StationId) => void; acts?: ReactNode }) {
  const parent = crumbs[crumbs.length - 2]?.label ?? "Cockpit";
  return (
    <section className="proto-layerbar" aria-label="Layer bar">
      <Tooltip label="Back">
        <Button variant="secondary" size="sm" aria-label={`Back to ${parent}`} onClick={onBack}>
          <ChevronLeft size={14} aria-hidden />
          {parent}
          <Kbd>Esc</Kbd>
        </Button>
      </Tooltip>
      <ol className="proto-crumbs" aria-label="Where">
        {crumbs.map((one, at) => (
          <li key={`${at}:${one.label}`} className="proto-crumbs__one" data-here={at === crumbs.length - 1 || undefined}>
            {at === 0 ? null : <ChevronRight size={12} aria-hidden />}
            {at === crumbs.length - 1 ? <span aria-current="page">{one.label}</span> : (
              <button type="button" className="proto-crumbs__press" onClick={one.press}>
                {one.label}
              </button>
            )}
          </li>
        ))}
      </ol>
      {acts}
      <RollCall stations={stations} onPress={onPress} />
    </section>
  );
}

/** What the shell draws at the top of its mount, on every page: the layer bar, and the calls over everything. */
export function AppChrome(props: AppSeamProps) {
  const ship: Ship = useShip(props);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const calls = useCalls(props, ship);
  const sessions = useSessions();
  const stations = stationsOf(ship);
  const latest = useRef(props);
  latest.current = props;
  placeHost.current = props.goTo;
  useEffect(() => setCardOpen(calls.open), [calls.open]);
  const cockpit = props.showing === SURFACE.overview && props.openJobId === null;
  const place = `${props.showing}|${props.openJobId}|${props.openSessionId}`;
  const here = useRef(place);
  here.current = place;
  const atCockpit = useRef(cockpit);
  atCockpit.current = cockpit;
  const cardOpen = useRef(calls.open);
  cardOpen.current = calls.open;

  // `g` and a key goes to a place; `w` opens the call that arrived, over whatever is open.
  const armed = useRef<number | undefined>(undefined);
  useListKeydown((event) => {
    if (event.metaKey || event.ctrlKey || event.altKey || holdsText(event.target)) return;
    if (chord.armed) {
      window.clearTimeout(armed.current);
      chord.armed = false;
      const to = CHORD[event.key];
      if (to === undefined) return;
      chord.took = event;
      event.preventDefault();
      props.goTo(to);
      return;
    }
    if (calls.open) return;
    // The merge line's real page has rows to read and no Merge, so a ready pull request is merged from the bar.
    if (event.key === "m" && latest.current.showing === SURFACE.mergeLine) {
      const ready = ship.subjects.find((one) => one.mergeable);
      if (ready !== undefined) {
        event.preventDefault();
        acts.merge(ready);
      }
      return;
    }
    if (event.key === "g") {
      chord.armed = true;
      armed.current = window.setTimeout(() => (chord.armed = false), 1500);
      event.preventDefault();
    } else if (event.key === "w") {
      event.preventDefault();
      calls.openFront();
    }
  });
  // Escape pops one layer, where the page under it did not take the press for itself.
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || atCockpit.current || cardOpen.current) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null || holdsText(event.target)) return;
      const before = here.current;
      window.setTimeout(() => {
        if (here.current === before && !atCockpit.current) latest.current.back();
      }, 80);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const crumbs: { label: string; press: () => void }[] = [{ label: "Cockpit", press: () => props.goTo(SURFACE.overview) }];
  if (props.showing !== SURFACE.overview) crumbs.push({ label: NAME[props.showing] ?? props.showing, press: () => props.goTo(props.showing) });
  const session = props.openSessionId === null || props.showing !== SURFACE.sessions ? undefined : sessions.find((one) => one.id === props.openSessionId);
  if (session !== undefined) crumbs.push({ label: session.title ?? session.id, press: () => undefined });
  const job = props.openJobId === null ? undefined : props.state.jobs.find((one) => one.id === props.openJobId);
  if (props.openJobId !== null) crumbs.push({ label: job?.title ?? "Job", press: () => undefined });
  const jump = (id: StationId) => {
    ui.setPin(id);
    if (!atCockpit.current) props.goTo(SURFACE.overview);
  };
  return (
    <>
      {cockpit ? null : (
        <LayerBar
          crumbs={crumbs}
          stations={stations}
          onBack={props.back}
          onPress={jump}
          acts={
            props.showing !== SURFACE.mergeLine ? undefined : (
              <span className="proto-layerbar__acts">
                {ship.subjects
                  .filter((one) => one.mergeable && one.pr !== undefined)
                  .map((one) => (
                    <Button key={one.key} variant="secondary" size="sm" aria-label={`Merge #${one.pr!.number}`} onClick={() => acts.merge(one)}>
                      Merge #{one.pr!.number}
                      <Kbd>{KEYS.merge}</Kbd>
                    </Button>
                  ))}
              </span>
            )
          }
        />
      )}
      <div className="armada-cockpit proto-app-overlay" data-layer={!cockpit || undefined}>
        {calls.tray}
        {calls.card}
      </div>
    </>
  );
}

// The Cockpit as a flight deck. A left column of the repository and every Session with the feed under it; a stage
// that is the timeline until something is picked and then that thing (a Session's thread, a bay, a pull
// request, a call, Autopilot's lane); the merge queue under it; the bays on the right; and along the
// foot the flight controls and Autopilot. Esc returns the stage to what it was. The state is held in `store.ts`, so the top bar,
// the body and the keys are one thing. The stage at rest is the map (the battlefield) or the timeline, one switch. Ported from
// proto/cockpit-flight-deck. #2301.

import { memo, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
import { ChartNoAxesGantt, CircleDashed, FileCog, GitMerge, Link, LoaderCircle, Pause, Plane, Play, Power, Send, Trash2, Unplug, UserCheck } from "lucide-react";
import { Badge, Button, ClashMark, ClashesProvider, DockHandle, DropdownMenu, Inspect, Kbd, EarlierMark, HandoffMark, PausedMark, Switch, Tooltip, keyFor, useMarking, type Clashing, type Mark } from "@armada/components";
import type { RepositorySummary, WorktreeSlot } from "@armada/protocol";
import { FIND_A_REPOSITORY, jobIdOf, manifestLabel, repositoryEntries, rootOf, useLayout, useRecentRepositories } from "@armada/shell";

import type { Item } from "../../Dashboard";
import { RepositoryCombobox } from "./RepositoryCombobox";
import { clashingOf, useWriteClashes } from "../../clashes";
import { baysOf } from "../../session-view/slot-trouble";
import { askToOpenHolder } from "../../open-holder";
import { useDispatching } from "../../dispatch-panel/dispatching";
import { viewsOf } from "../../merge-line";
import { sessionHolds } from "./session-prs";
import { askToOpenSessionInCockpit } from "../../open-session";
import { useQueued, useSessions, useSessionsDraft } from "../../sessions-draft";
import { useAutopilotLane } from "../AutopilotLane";
import { useColumns } from "./columns";
import { useSplits } from "./splits";
import { dotsOf, MERGE_BY_HAND } from "../horizon";
import { useFleetEvents } from "../fleet-events";
import { bandingOf, bandsOf, type Banding } from "./bands";
import { bandOf, movableIn, useReorder } from "./reorder";
import { useClock } from "./clock";
import { CLOCK_MS } from "./field";
import { ActivityGraph } from "../ActivityChart";
import { Timeline } from "../Timeline";
import type { CockpitProps } from "./props";
import { useShipActs } from "./acts";
import { Bays } from "./BayRows";
import { BuildColumn } from "./BuildColumn";
import { chipPreview } from "./chip-card";
import { SubjectCard } from "./inspect-cards";
import { glyphOf, wordOf, type BuildIn } from "./build";
import { useBuilds } from "./useBuilds";
import { DECK_LINK } from "./links";
import { holderKeyOf, useBays, slotKey } from "./bays";
import { useCollapse } from "./collapse";
import { PanelKey, panelAt } from "./keys";
import { queuedIn, queuedKey } from "./queued";
import type { QueuedIdea } from "@armada/screens/src/draft/sessions";
import { MapView } from "./MapPane";
import { CallPane, PaneHead, SessionPane } from "./panes";
import { inOrder, useReplayDriver, useReplayKeys, windowOf } from "./replay";
import { RowMenu, type RowMenuAt } from "./RowMenu";
import { FlightEdit } from "./FlightEdit";
import { isWebAddress, tilesOf, useFlightFile, type FlightSet } from "./flight-controls";
import { glyphOf as flightGlyph } from "./flight-glyphs";
import { SessionTiles } from "./SessionTiles";
import { reportRoot } from "./session-tiles";
import { checkoutSaid, heldOf } from "./YourCheckout";
import { forgeOf, quietSaid, ReadAgo } from "./forge";
import { useListed } from "./listed";
import type { Found } from "./find";
import { MainLight } from "./MainLight";
import { MarkGlyph } from "./shared";
import { toneOf } from "./order";
import { callOwnerOf, enterAutopilot, incomingOf, keepOf, needingOf, leaveAutopilot, moveCursor, moveRow, openCall, openLinked, openSubject, select, selOfSubject, setFocus, setHover, setView, setQuery, setScope, setSearching, showMoreRows, toggleChecks, toggleNeeds, useDeck, withoutOpenSession, SCOPES, type DeckState, type Scope } from "./store";
import { callingHomes } from "./homes";
import { useShip, type Ship, type Subject } from "./ship";
import type { Hosts } from "../../session-view/SessionWorkspace";
import { openPullRequestInPanel, openSlotKeyInPanel, usePanel } from "../../shared-panel/store";
import "../cockpit.css";
import "./deck.css";


/** The slots the pool serves, narrowed to the picked repository as the ship's are. */
export const slotsOf = (props: Pick<CockpitProps, "state" | "picked">): readonly WorktreeSlot[] =>
  props.state.held.state === "read" ? (props.state.held.held.slots ?? []).filter((one) => props.picked === null || jobIdOf(props.picked) === undefined || one.manifest_id === jobIdOf(props.picked)) : [];

const repoNameOf = (props: Pick<CockpitProps, "state">, manifest: string) => manifestLabel(manifest, props.state.holds.repositories ?? []);

// ---- The left column ----------------------------------------------------------------------------------

/** The repository, at the head of the left column: it narrows the list, the feed, the bays and the queue, as the title row's picker does. */
function RepositoryScope({ props }: { props: CockpitProps }) {
  const repositories = props.state.holds.repositories ?? [];
  const pick = props.onPickRepository;
  const add = props.onAddRepository;
  if (props.onGoTo === undefined && (repositories.length === 0 || pick === undefined)) return null;
  return (
    <div className="deck-scope">
      {repositories.length === 0 || pick === undefined ? null : (
        <RepositoryCombobox
          repositories={repositories}
          picked={props.picked}
          onPick={(root) => pick(root)}
          onAdd={add}
        />
      )}
      {props.onGoTo === undefined ? null : (
        <Tooltip label="Open the Manifest" asChild>
          <Button variant="ghost" size="sm" iconOnly aria-label="Open the Manifest" disabled={repositories.length === 0} onClick={() => props.onGoTo?.(DECK_LINK.picker)}>
            <FileCog size={16} aria-hidden="true" />
          </Button>
        </Tooltip>
      )}
    </div>
  );
}

/** Where a Session is in its build, as the one glyph of its phase. A Session with no build has an empty cell. */
function Phase({ subject, builds }: { subject: Subject; builds: ReadonlyMap<string, BuildIn> }) {
  const build = subject.kind === "session" ? builds.get(subject.key) : undefined;
  const Glyph = build !== undefined ? glyphOf(build) : undefined;
  const word = build !== undefined ? wordOf(build) : undefined;
  if (Glyph === undefined || word === undefined) return <span className="deck-row__phase" />;
  return (
    <Tooltip label={word}>
      <span className="deck-row__phase" role="img" aria-label={word}>
        <Glyph size={14} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** An idea saved to start later, a row of the list: dashed, with its repository and model. ↵ (a click with no pointer) or its play press starts it, never a mouse click on its words; Remove lets it go. */
export function QueuedRow({ idea }: { idea: QueuedIdea }) {
  const draft = useSessionsDraft();
  const cursor = useDeck((deck) => deck.cursor);
  const hover = useDeck((deck) => deck.hover);
  const key = queuedKey(idea.id);
  const title = idea.text.trim().split("\n")[0] ?? "";
  const start = () => {
    if (draft?.startQueued === undefined) return;
    void Promise.resolve(draft.startQueued(idea.id)).then((session) => session !== undefined && askToOpenSessionInCockpit(session));
  };
  return (
    <li className="deck-list__item">
      <button type="button" className="deck-row" data-row={key} data-queued data-cursor={cursor === key || undefined} data-hover={hover === key || undefined} aria-label={title} onClick={(event) => event.detail === 0 && start()} onFocus={() => (moveCursor(key), setHover(key))} onBlur={() => setHover(undefined)} onMouseEnter={() => setHover(key)} onMouseLeave={() => setHover(undefined)}>
        <Tooltip label="Queued">
          <span className="deck-row__kind" role="img" aria-label="Queued">
            <CircleDashed size={14} aria-hidden="true" />
          </span>
        </Tooltip>
        <span className="deck-row__title">{title}</span>
        <span className="deck-row__tags">
          <Tooltip label="Repository">
            <span role="img" aria-label={`Repository ${idea.repository}`}>
              {idea.repository}
            </span>
          </Tooltip>
          {idea.model === undefined ? null : (
            <Tooltip label="Model">
              <span role="img" aria-label={`Model ${idea.model}`}>
                {idea.model}
              </span>
            </Tooltip>
          )}
        </span>
      </button>
      <Tooltip label="Start" asChild>
        <Button variant="ghost" size="sm" iconOnly aria-label={`Start ${title}`} onClick={start}>
          <Play size={16} aria-hidden="true" />
        </Button>
      </Tooltip>
      <Tooltip label="Remove" asChild>
        <Button variant="ghost" size="sm" iconOnly aria-label={`Remove ${title}`} onClick={() => draft?.dropQueued?.(idea.id)}>
          <Trash2 size={16} aria-hidden="true" />
        </Button>
      </Tooltip>
    </li>
  );
}

/** Draws a page more of the list each time its end comes into view. */
function MoreRows() {
  const mark = useRef<HTMLLIElement>(null);
  useEffect(() => {
    const at = mark.current;
    if (at === null || typeof IntersectionObserver === "undefined") return;
    const seen = new IntersectionObserver((entries) => {
      if (entries.some((one) => one.isIntersecting)) showMoreRows();
      // Drawn a little before the end is reached, so a row just above it comes with its next page.
    }, { root: at.closest(".deck-list__rows"), rootMargin: "0% 0% 50% 0%" });
    seen.observe(at);
    return () => seen.disconnect();
  }, []);
  return (
    <li className="deck-list__more" ref={mark} role="status" aria-label="Loading more Jobs">
      <LoaderCircle size={14} strokeWidth={2} aria-hidden />
    </li>
  );
}

/** The words the Filter trigger names the narrowing with, cut short. */
const cut = (words: string, at: number) => (words.length > at ? `${words.slice(0, at).trimEnd()}…` : words);

/**
 * **Filter**, at the right of the list's band: plain text and a chevron that open a small menu. A search field heads it with the caret in
 * it, and under it are the four scopes, the current one checked. ↓ walks from the field to the scopes and Enter in the field goes to the
 * rows; Esc closes it. While the list is narrowed, by a scope but Active or by search words, the band says so in place of "Filter".
 * ↑ from the first row opens it.
 */
function Filter({ rows }: { rows: RefObject<HTMLElement | null> }) {
  const typed = useDeck((deck) => deck.query);
  const query = typed.trim();
  const scope = useDeck((deck) => deck.scope);
  const open = useDeck((deck) => deck.searching);
  const named = scope === "active" ? undefined : SCOPES.find((one) => one.id === scope)?.label;
  const said = named === undefined ? (query === "" ? undefined : cut(query, 14)) : query === "" ? named : `${named} · ${cut(query, 8)}`;
  return (
    <DropdownMenu
      align="end"
      portal
      plain
      triggerLabel={said ?? "Filter"}
      {...(said === undefined ? {} : { triggerAriaLabel: `Filter: ${said}` })}
      searchLabel="Search Jobs"
      search={{ value: typed, onChange: setQuery, onEnter: () => rows.current?.querySelector<HTMLElement>(".deck-row")?.focus() }}
      open={open}
      onOpenChange={setSearching}
      entries={SCOPES.map(({ id, label }) => ({ kind: "item" as const, id, label, selected: id === scope }))}
      onSelect={(id) => setScope(id as Scope)}
    />
  );
}

/** What a linked Session's link glyph is called: the plan group it was handed, where it was. */
const linkedSaid = (linked: { group?: string }) => (linked.group === undefined ? "Linked Job" : `Linked Job, handed from ${linked.group}`);

/** What each thing a Session waits on is called, by the item's source. */
const CALL_WORD: Record<string, string> = { ask_card: "Question", window_ask: "Question", permission: "Permission", plan: "Plan", judge: "Judge concern", slot_choice: "Slot", walk: "Walk", done: "Done" };

/** What a call says, as the tooltip on its row: what it is, then its words. */
const callLabel = (call: Item): string => {
  const kind = call.kind.replace(/^Job ?/, "");
  const word = CALL_WORD[call.waiting?.item.source ?? ""] ?? (kind === "" ? "Waiting" : kind.charAt(0).toUpperCase() + kind.slice(1));
  return call.fact === "" ? word : `${word}: ${call.fact}`;
};

function List({ subjects, calling, banding, selected, builds, picked, found, onPick, onEarlier, onMove, onAutopilot }: { subjects: readonly Subject[]; calling: ReadonlyMap<string, Item> | undefined; banding: readonly Banding[] | undefined; selected: string | undefined; builds: ReadonlyMap<string, BuildIn>; picked: RepositorySummary | null; found: ReadonlyMap<string, Found> | undefined; onPick: (subject: Subject) => void; onEarlier: (id: string) => void; onMove: (key: string, target: string, after: boolean) => void; onAutopilot: (root: string) => void }) {
  const folded = useCollapse("list");
  const cursor = useDeck((deck) => deck.cursor);
  const hover = useDeck((deck) => deck.hover);
  const limit = useDeck((deck) => deck.limit);
  const ideas = useQueued();
  const pausedIds = new Set(useSessions().filter((one) => one.paused === true).map((one) => one.id));
  const [menu, setMenu] = useState<RowMenuAt>();
  const panel = useRef<HTMLElement>(null);
  const drawn = subjects.slice(0, limit);
  // A live Session is picked up by its row and put down among those under its heading.
  const movable = movableIn(subjects);
  const { dragging, grab, swallowed } = useReorder(onMove);
  const row = (one: Subject) => {
    const band = movable.has(one.key) ? bandOf(subjects, banding, one.key) : undefined;
    const quiet = one.since ?? "";
    const said = quiet.length > 9 ? "" : quiet;
    // What a search matched in the Session, in place of its age.
    const hit = found?.get(one.key);
    // A Session calling him: its row pulses a bar, and its tooltip names the call.
    const call = calling?.get(one.key);
    // Needs him: the row is the ask hue and pulses while it does. The open Session's own calls are in its thread, so its row is quiet.
    // The row stands for the earlier Job too: it is open when that one is.
    const here = selected === one.key || (one.earlier !== undefined && selected === `session:${one.earlier}`);
    const needs = one.kind === "session" && toneOf(one) === "ask" && !here;
    return (
      <li key={one.key} className="deck-list__item" data-movable={band} data-lifted={dragging?.key === one.key || undefined} data-drop={dragging?.drop?.key === one.key ? (dragging.drop.after ? "after" : "before") : undefined} onContextMenu={one.kind === "session" ? (event) => (event.preventDefault(), setMenu({ id: one.id, root: one.root, x: event.clientX, y: event.clientY, row: event.currentTarget.querySelector<HTMLElement>(".deck-row") })) : undefined}>
        <Inspect card={<SubjectCard one={one} {...(call === undefined ? {} : { calling: callLabel(call) })} />} name={one.title} side="beside" pin={false}>
        <button type="button" className="deck-row" data-row={one.key} data-calling={call === undefined && !needs ? undefined : ""} data-needs={needs || undefined} data-linked={one.linked !== undefined || undefined} data-reply={one.reply} data-tone={toneOf(one)} data-past={one.past} data-selected={here || undefined} data-cursor={cursor === one.key || undefined} data-hover={hover === one.key || undefined} aria-label={one.title} aria-current={here || undefined} onPointerDown={band === undefined ? undefined : (event) => grab(event, one.key, band)} onClickCapture={(event) => swallowed() && (event.preventDefault(), event.stopPropagation())} onClick={() => onPick(one)} onFocus={() => (moveCursor(one.key), setHover(one.key))} onBlur={() => setHover(undefined)} onMouseEnter={() => setHover(one.key)} onMouseLeave={() => setHover(undefined)} onKeyDown={one.kind === "session" ? (event) => {
          // The earlier Job behind this row, where it took one's place. Not the registry's yet: the key waits on the look.
          if (event.key === "h" && one.earlier !== undefined && !event.metaKey && !event.ctrlKey && !event.altKey) return (event.preventDefault(), onEarlier(one.earlier));
          if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
          event.preventDefault();
          const box = event.currentTarget.getBoundingClientRect();
          setMenu({ id: one.id, root: one.root, x: box.left + box.width / 2, y: box.bottom, row: event.currentTarget });
        } : undefined}>
          <span className="deck-row__title">
            {one.linked === undefined ? null : (
              <Tooltip label={linkedSaid(one.linked)}>
                <span className="deck-row__link" role="img" aria-label={linkedSaid(one.linked)}>
                  <Link size={12} aria-hidden="true" />
                </span>
              </Tooltip>
            )}
            {one.title}
          </span>
          <span className="deck-row__marks">
            <span className="deck-row__clash-slot" aria-hidden="true" />
            {one.handoff === "starting" ? <HandoffMark /> : pausedIds.has(one.id) ? <PausedMark said="Paused" /> : <Phase subject={one} builds={builds} />}
            <MarkGlyph mark={{ ...one.mark, tone: toneOf(one) }} size={one.reply === true ? 12 : 16} />
          </span>
          <span className="deck-row__says">{one.peek.at(-1) ?? ""}</span>
          {hit === undefined || hit.short === "" ? (
            <span className="deck-row__said">{said}</span>
          ) : (
            <Tooltip label={hit.whole}>
              <span className="deck-row__said" data-found="" role="img" aria-label={hit.whole}>
                {hit.short}
              </span>
            </Tooltip>
          )}
        </button>
        </Inspect>
        {one.earlier === undefined ? null : (
          <span className="deck-row__earlier">
            <EarlierMark onOpen={() => onEarlier(one.earlier!)} />
          </span>
        )}
        {one.kind === "pr" ? null : (
          <span className="deck-row__clash">
            <ClashMark kind={one.kind} id={one.id} />
          </span>
        )}
      </li>
    );
  };
  return (
    // A press on the list and on no row lets the cursor go.
    <section ref={panel} className="deck-panel deck-list" aria-label="Jobs" tabIndex={-1} data-collapsed={folded.collapsed || undefined} onClick={(event) => (event.target as Element).closest(".deck-row") === null && moveCursor(undefined)}>
      <header className="deck-band" {...folded.band}>
        Jobs
        <Filter rows={panel} />
        <PanelKey id="list" />
      </header>
      <ul className="deck-list__rows" data-reordering={dragging === undefined ? undefined : ""}>
        {banding === undefined
          ? [...queuedIn(ideas, picked).map((idea) => <QueuedRow key={queuedKey(idea.id)} idea={idea} />), ...drawn.map(row)]
          : bandsOf(drawn, banding, ideas).map((band) => (
              <li key={`band:${band.key}`} className="deck-list__band">
                <h3 className="deck-list__repository">{band.label}</h3>
                <ul className="deck-list__group">{[...band.ideas.map((idea) => <QueuedRow key={queuedKey(idea.id)} idea={idea} />), ...band.subjects.map(row)]}</ul>
              </li>
            ))}
        {drawn.length < subjects.length ? <MoreRows key={limit} /> : null}
      </ul>
      <RowMenu at={menu} onClose={() => setMenu(undefined)} onAutopilot={onAutopilot} />
    </section>
  );
}

/**
 * Activity stands as tall as the merge queue and the flight controls beside it: from the queue's top to the foot's
 * bottom. It is in a side column, so the height is set on the grid and it reads it. A size he has dragged to takes over from it.
 */
export function useActivityBeside(grid: RefObject<HTMLDivElement | null>, layout: string) {
  useEffect(() => {
    const root = grid.current;
    if (root === null || typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(() => measure());
    const measure = () => {
      const queue = root.querySelector<HTMLElement>(".deck-queue");
      const foot = root.querySelector<HTMLElement>(".deck-foot");
      if (queue === null || foot === null) return;
      // The queue is redrawn when the repository changes, so the one on screen is watched each time.
      watch.observe(queue);
      const height = foot.getBoundingClientRect().bottom - queue.getBoundingClientRect().top;
      if (height > 0) root.style.setProperty("--deck-activity", `${Math.round(height)}px`);
    };
    for (const one of root.querySelectorAll(".deck-centre, .deck-foot, .deck-stage > *")) watch.observe(one);
    measure();
    return () => watch.disconnect();
  }, [grid, layout]);
}

/**
 * What Fleet has been doing over the last hour, as a beeswarm with no words: a dot for each event. **Hovering a dot shows the events
 * near it and pressing it picks its event**, and the map shows what moved: the drone it is about, ringed, and the rest dimmed.
 */
function Activity({ props, listed, ship }: { props: CockpitProps; listed: readonly Subject[]; ship: Ship }) {
  const needs = useDeck((deck) => deck.needs);
  const checks = useDeck((deck) => deck.checks);
  const sel = useDeck((deck) => deck.sel);
  const focus = useDeck((deck) => deck.focus);
  const replay = useDeck((deck) => deck.replay);
  const views = useMemo(() => viewsOf(props.state), [props.state]);
  const scope = useDeck((deck) => deck.scope);
  const query = useDeck((deck) => deck.query);
  const keep = useMemo(() => keepOf(listed, { needs, checks, scope, query }), [listed, needs, checks, scope, query]);
  // The event held lit: the one picked, or the one the playhead has last passed. Its column is marked.
  const events = useFleetEvents();
  // A Session open on the stage is the only one the graph tells of.
  const open = sel?.kind === "session" ? ship.subjects.find((one) => one.key === sel.key && one.kind === sel.kind) : undefined;
  const of = useMemo(() => (open === undefined || open.kind === "pr" ? undefined : { kind: open.kind, id: open.id }), [open?.kind, open?.id]);
  const lit = useMemo(() => {
    if (replay === undefined) return focus;
    const { at } = replay;
    return inOrder(events).filter((one) => Date.parse(one.at) <= at).at(-1)?.id.toString();
  }, [events, focus, replay === undefined ? undefined : replay.at]);
  return (
    <section className="deck-panel deck-activity" aria-label="Activity" tabIndex={-1}>
      <header className="deck-band">Activity</header>
      <ActivityGraph
        now={props.now}
        views={views}
        filter="running"
        keep={keep as never}
        {...(of === undefined ? {} : { of })}
        onPick={(event) => setFocus(event.id, event.subject.kind === "main" ? undefined : `${event.subject.kind}:${event.subject.id}`)}
        {...(lit === undefined ? {} : { lit })}
      />
    </section>
  );
}

// ---- The stage ----------------------------------------------------------------------------------------

/** The timeline, the stage's rest. It keeps the same filters as the list. */
function TimelineView({ props, ship, listed }: { props: CockpitProps; ship: Ship; listed: readonly Subject[] }) {
  const views = useMemo(() => viewsOf(props.state), [props.state]);
  const needs = useDeck((deck) => deck.needs);
  const checks = useDeck((deck) => deck.checks);
  const scope = useDeck((deck) => deck.scope);
  const query = useDeck((deck) => deck.query);
  const keep = useMemo(() => keepOf(listed, { needs, checks, scope, query }), [listed, needs, checks, scope, query]);
  const pick = (key: string) => {
    const subject = ship.subjects.find((one) => one.key === key);
    if (subject !== undefined && subject.kind !== "pr") select(selOfSubject(subject));
  };
  return (
    <section className="deck-pane" aria-label="Fleet timeline">
      <header className="deck-band deck-pane__head">
        <ChartNoAxesGantt size={16} aria-hidden="true" />
        <h2 className="deck-pane__title">Timeline</h2>
      </header>
      <div className="deck-timeline">
        <Timeline
          now={props.now}
          views={views}
          filter="running"
          recessed={false}
          keep={keep as never}
          onOpenSession={(id) => pick(`session:${id}`)}
        />
      </div>
    </section>
  );
}

/** The bays, as views: the pool's own states and who is behind each, with the holders the slot events name. */
function useBayViews(props: CockpitProps) {
  const events = useFleetEvents();
  const slots = slotsOf(props);
  return useBays(slots, events, undefined, props.now);
}

function Stage({ props, ship, listed, autopilot, ledger, hosts }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; autopilot: ReturnType<typeof useAutopilotLane>; ledger: HTMLElement | null; hosts: Hosts | undefined }) {
  const sel = useDeck((deck) => deck.sel);
  const view = useDeck((deck) => deck.view);
  const acts = useShipActs(props);
  const subject = sel === undefined ? undefined : ship.subjects.find((one) => one.key === sel.key && one.kind !== "pr");
  const call = sel?.kind === "call" ? ship.calls.find((one) => one.key === sel.key) : undefined;
  // What was picked is gone (a Session ended, a call was answered elsewhere): back to the timeline.
  const lane = sel?.kind === "autopilot";
  const gone = sel !== undefined && !lane && subject === undefined && call === undefined;
  useEffect(() => {
    if (gone) select(undefined);
  }, [gone]);
  // A Job handed to a new one is followed to it, so the stage keeps its place. Only where it was a row a moment ago: the earlier Job opened from its row's mark was never one, and stays open.
  const rows = useRef<ReadonlySet<string>>(new Set());
  const took = subject?.behind === true && rows.current.has(subject.key) ? ship.subjects.find((one) => one.kind === "session" && `session:${one.earlier}` === subject.key) : undefined;
  useEffect(() => {
    if (took !== undefined) select(selOfSubject(took));
  }, [took]);
  useEffect(() => {
    rows.current = new Set(ship.subjects.filter((one) => one.behind !== true).map((one) => one.key));
  });
  // A move makes its Session a moment before it opens it, so the ship is read as it stands then and not as it stood when pressed.
  const latest = useRef(ship);
  latest.current = ship;
  // A Session this ship does not hold opens on its own page, as a link to one does.
  const openSession = (id: string) => {
    const held = latest.current.subjects.find((one) => one.kind === "session" && one.id === id);
    if (held === undefined) props.onOpenSession(id);
    else openSubject(latest.current, held);
  };
  if (sel?.kind === "session" && subject !== undefined) return <SessionPane subject={subject} props={props} acts={acts} ledger={ledger} hosts={hosts} openSession={openSession} />;
  if (call !== undefined) return <CallPane item={call} props={props} />;
  if (lane) return <AutopilotPane props={props} autopilot={autopilot} />;
  if (view === "map") return <MapView props={props} ship={ship} listed={listed} />;
  return <TimelineView props={props} ship={ship} listed={listed} />;
}

// ---- The merge queue ----------------------------------------------------------------------------------

/** The repository a Session open on the stage belongs to, by the root its line is served under. */
function queueRootOf(sel: DeckState["sel"], ship: Ship): string | undefined {
  if (sel?.kind === "session") return ship.subjects.find((one) => one.key === sel.key && one.kind === sel.kind)?.root;
  return undefined;
}

/**
 * The merge queue: one row across the repositories, main first and then one chip for each pull request, each repository's
 * nearest main first. Main is one light, green while every main is and red if any is; a red one opens a card with the Merge line's
 * panel for each red (`MainLight`), and a mark on it with the checkout section while a served checkout is held behind main. A press on a chip opens the pull request on the stage. No count.
 */
function MergeQueue({ props, ship }: { props: CockpitProps; ship: Ship }) {
  const sessions = useSessions();
  const answered = useDeck((deck) => deck.held);
  const seen = useDeck((deck) => deck.visited);
  // A call no Session's row holds pulses the chip it is about, as a calling row does.
  const homes = callingHomes(ship.calls, answered, seen, sessions);
  const sel = useDeck((deck) => deck.sel);
  const panel = usePanel().open;
  const served = viewsOf(props.state);
  // A layout that hides the merge line's panel hides this queue, its one counterpart.
  const shown = useLayout("dashboard.panels").shown.some((one) => one.id === "merge-line");
  if (served.length === 0 || !shown) return null;
  // On All with several repositories served, the queue is the open Session's or pull request's repository, and every one's, by repository, with none.
  const wide = props.picked === null && served.length > 1;
  const root = wide ? queueRootOf(sel, ship) : undefined;
  const scoped = root === undefined ? [] : served.filter((view) => view.root === root);
  const views = scoped.length > 0 ? scoped : served;
  const nameOf = (root: string) => ship.repos.find((one) => one.root === root)?.name ?? root;
  const merge =
    props.onGoTo === undefined ? (
      <GitMerge size={16} aria-hidden="true" />
    ) : (
      <Tooltip label="Merge line" asChild>
        <Button variant="ghost" size="sm" iconOnly aria-label="Merge line" onClick={() => props.onGoTo?.(DECK_LINK.queue)}>
          <GitMerge size={16} aria-hidden="true" />
        </Button>
      </Tooltip>
    );
  // One light for the line, `main`; on All with several repositories, one each, named for its repository and leading its pull requests.
  const grouped = views.length > 1;
  const summoner = views.find((view) => view.hub?.checkout !== undefined)?.root;
  const lightOf = (group: typeof views, name: string, summon: boolean) => {
    const reds = group.flatMap((view) => (view.hub?.main?.state !== "red" ? [] : [{ view: { ...view, hub: view.hub }, name: nameOf(view.root) }]));
    const held = heldOf(group, nameOf);
    const calling = group.some((view) => homes.has(`main:${view.root}`) || homes.has(`checkout:${view.root}`));
    const red = reds.map((one) => one.name);
    const forge = forgeOf(group, nameOf);
    const quiet = quietSaid(forge, group.length > 1);
    const where = grouped ? ` in ${name}` : "";
    const mainSaid = `${red.length > 0 ? (grouped ? `Main is red${where}` : `Main is red in ${red.join(", ")}`) : `Main is green${where}`}${held.length > 0 ? `, ${checkoutSaid(held).replace(/^Checkout/, "checkout")}` : ""}`;
    return reds.length > 0 || held.length > 0 ? (
      <MainLight props={props} ship={ship} reds={reds} held={held} said={mainSaid} forge={forge} quiet={quiet} name={name} summon={summon} calling={calling} />
    ) : (
      <Tooltip label={forge.readAt === undefined ? mainSaid : <>{mainSaid}. <ReadAgo at={forge.readAt} /></>} asChild>
        <span className="deck-chip" data-main role="img" aria-label={mainSaid}>
          <span className="deck-chip__dot" data-tone="calm" data-stale={quiet !== undefined || undefined} aria-hidden="true" />
          <span>{name}</span>
          {quiet === undefined ? null : (
            <Tooltip label={quiet}>
              <span className="deck-chip__quiet" role="img" aria-label={quiet}>
                <Unplug size={12} aria-hidden="true" />
              </span>
            </Tooltip>
          )}
        </span>
      </Tooltip>
    );
  };
  // While a Session is open its pull requests are framed in the line; with none open, or a pull request, none are.
  const viewed = sel?.kind === "session" ? sessions.find((one) => `session:${one.id}` === sel.key) : undefined;
  const prOfDot = (view: (typeof views)[number], dot: ReturnType<typeof dotsOf>[number]) => ship.prs.find((one) => one.root === view.root && `pull:${one.number}` === dot.key);
  const relatedDot = (view: (typeof views)[number], dot: ReturnType<typeof dotsOf>[number]) => {
    const pr = prOfDot(view, dot);
    return viewed !== undefined && pr !== undefined && sessionHolds(viewed, pr);
  };
  const chipOf = (view: (typeof views)[number], dot: ReturnType<typeof dotsOf>[number]) => {
    const pr = prOfDot(view, dot);
    const related = relatedDot(view, dot);
    const owner = pr === undefined ? undefined : ship.subjects.find((one) => one.pr?.key === pr.key);
    const tone = pr?.stage === "red" ? "bad" : owner?.mergeable === true ? "ask" : "calm";
    const place = dot.queued ? dot.card.place : undefined;
    const inside = (
      <>
        {place === undefined ? null : <span className="deck-chip__place" aria-hidden="true">{place}</span>}
        <span className="deck-chip__dot" data-state={dot.state} data-queued={dot.queued || undefined} data-tone={tone} aria-hidden="true" />
        <span>{dot.card.heading}</span>
        {dot.card.byHand === undefined ? null : (
          <Tooltip label={MERGE_BY_HAND}>
            <span className="deck-chip__hand" role="img" aria-label={MERGE_BY_HAND}>
              <UserCheck size={12} aria-hidden="true" />
            </span>
          </Tooltip>
        )}
      </>
    );
    return (
      <li key={`${view.root}:${dot.key}`}>
        <Inspect card={chipPreview(dot, pr !== undefined)} name={dot.tip} pin={false}>
          {pr === undefined ? (
            <span className="deck-chip" role="img" aria-label={dot.tip}>
              {inside}
            </span>
          ) : (
            <button type="button" className="deck-chip" data-selected={(panel?.kind === "pr" && panel.number === pr.number && (panel.root === undefined || panel.root === pr.root)) || undefined} data-related={related || undefined} data-calling={homes.has(dot.key) ? "" : undefined} aria-label={dot.tip} onClick={() => openPullRequestInPanel(pr)}>
              {inside}
            </button>
          )}
        </Inspect>
      </li>
    );
  };
  // Each repository's queue first, in its order and framed as the queue, then its other pull requests past a rule.
  // **A draft is left off unless the open Session holds it, and the open Session's own pull requests lead the rest**,
  // so they stay in view however many others are open.
  const chipsOf = (view: (typeof views)[number]) => {
    const dots = dotsOf(view, props.state.mergeLines, sessions).filter((dot) => dot.card.draft !== true || dot.queued || relatedDot(view, dot));
    const queued = dots.filter((dot) => dot.queued).sort((a, b) => (a.card.place ?? Infinity) - (b.card.place ?? Infinity));
    const open = dots.filter((dot) => !dot.queued).sort((a, b) => Number(relatedDot(view, b)) - Number(relatedDot(view, a)));
    return [
      ...(queued.length === 0
        ? []
        : [
            <li key={`queue:${view.root}`}>
              <Tooltip label="Merge queue">
                <ol className="deck-queue__track" aria-label="Merge queue">
                  {queued.map((dot) => chipOf(view, dot))}
                </ol>
              </Tooltip>
            </li>,
          ]),
      ...(queued.length === 0 || open.length === 0 ? [] : [<li key={`rule:${view.root}`} className="deck-queue__rule" aria-hidden="true" />]),
      ...open.map((dot) => chipOf(view, dot)),
    ];
  };
  if (grouped) {
    const rows = views.map((view) => ({
      root: view.root,
      name: nameOf(view.root),
      red: view.hub?.main?.state === "red",
      chips: [<li key={`main:${view.root}`}>{lightOf([view], nameOf(view.root), view.root === summoner)}</li>, ...chipsOf(view)],
    }));
    return (
      <section className="deck-panel deck-queue" aria-label="Merge queue" tabIndex={-1}>
        <div className="deck-queue__row">
          {merge}
          <QueueStack rows={rows} />
          <PanelKey id="queue" />
        </div>
      </section>
    );
  }
  return (
    <section className="deck-panel deck-queue" aria-label="Merge queue" tabIndex={-1}>
      <div className="deck-queue__row">
        {merge}
        <ol className="deck-chips" onWheel={(event) => {
          // A wheel turned over the line moves it along, since it scrolls only across.
          if (event.deltaY !== 0 && event.deltaX === 0) event.currentTarget.scrollLeft += event.deltaY;
        }}>
          <li>{lightOf(views, "main", true)}</li>
          {views.flatMap((view, at) => [...(at === 0 ? [] : [<li key={`gap:${view.root}`} className="deck-queue__gap" aria-hidden="true" />]), ...chipsOf(view)])}
        </ol>
        <PanelKey id="queue" />
      </div>
    </section>
  );
}

/** How long the stacked line rests on one repository before it moves to the next, while nobody is using it. */
const ROTATE_MS = 15_000;

/**
 * On All, the merge line one repository at a time: each row scrolls across, and the stack scrolls down and snaps to a row.
 * **A main that turns red brings its row into view**, and while the pointer, the focus and the wheel are elsewhere the stack
 * moves to the next repository every `ROTATE_MS`. ↑ and ↓ move it by hand.
 */
function QueueStack({ rows }: { rows: readonly { root: string; name: string; red: boolean; chips: ReactNode[] }[] }) {
  const stack = useRef<HTMLDivElement>(null);
  const over = useRef(false);
  const touched = useRef(0);
  const seenRed = useRef(new Set<string>());
  const rowTo = (at: number) => {
    const host = stack.current;
    const row = host?.children[at] as HTMLElement | undefined;
    if (host === null || row === undefined) return;
    const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;
    host.scrollTo({ top: row.offsetTop - host.offsetTop, behavior: still ? "auto" : "smooth" });
  };
  const atNow = () => {
    const host = stack.current;
    return host === null || host.clientHeight === 0 ? 0 : Math.round(host.scrollTop / host.clientHeight);
  };
  const reds = rows.filter((row) => row.red).map((row) => row.root).join("\n");
  useEffect(() => {
    const now = new Set(rows.filter((row) => row.red).map((row) => row.root));
    const turned = rows.findIndex((row) => now.has(row.root) && !seenRed.current.has(row.root));
    seenRed.current = now;
    if (turned >= 0) rowTo(turned);
    // Only a change in which mains are red moves it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [reds]);
  useEffect(() => {
    const id = window.setInterval(() => {
      const host = stack.current;
      if (host === null || over.current || host.contains(document.activeElement) || Date.now() - touched.current < ROTATE_MS) return;
      rowTo((atNow() + 1) % rows.length);
    }, ROTATE_MS);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rows.length]);
  const touch = () => {
    touched.current = Date.now();
  };
  return (
    <div
      ref={stack}
      className="deck-queue__stack"
      onPointerEnter={() => {
        over.current = true;
      }}
      onPointerLeave={() => {
        over.current = false;
        touch();
      }}
      onWheel={touch}
      onKeyDown={(event) => {
        if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
        event.preventDefault();
        touch();
        rowTo(Math.min(rows.length - 1, Math.max(0, atNow() + (event.key === "ArrowDown" ? 1 : -1))));
      }}
    >
      {rows.map((row) => (
        <ol key={row.root} className="deck-chips" aria-label={row.name}>
          {row.chips}
        </ol>
      ))}
    </div>
  );
}

// ---- The right column, and the foot -------------------------------------------------------------------

/** A Session picked in the list narrows the bays to those it holds, or held last where stranded. Nothing picked, every bay. */
function BaysPanel({ props, ship, picked }: { props: CockpitProps; ship: Ship; picked: string | undefined }) {
  const bays = useBayViews(props);
  const working = useMemo(() => new Set(ship.subjects.filter((one) => one.phase === "working").map((one) => one.key)), [ship.subjects]);
  const shown = bays.map((one) => one.view).filter((view) => {
    if (picked === undefined) return true;
    return holderKeyOf(view) === picked;
  });
  const hovered = useDeck((deck) => deck.hover);
  const panel = usePanel().open;
  const selected = panel?.kind === "bay" ? slotKey({ manifest_id: panel.manifestId, slot: panel.slot }) : undefined;
  const change = props.bayActs?.onChange;
  const addSlot = change === undefined ? undefined : (manifest: string) => change(manifest, { act: "add" });
  return <Bays views={shown} hovered={hovered} onHover={setHover} repoName={(manifest) => repoNameOf(props, manifest)} working={working} selected={selected} onSelect={(key) => void openSlotKeyInPanel(key)} onOpenSlots={props.onGoTo === undefined ? undefined : () => props.onGoTo?.(DECK_LINK.bays)} onAddSlot={addSlot} />;
}

/** A Session open on the stage keeps its ledger here, where the Bays are otherwise. The pane draws the ledger into `host`. */
function LedgerPanel({ host }: { host: (element: HTMLElement | null) => void }) {
  return (
    <section className="deck-panel deck-ledger" aria-label="Ledger" tabIndex={-1}>
      <header className="deck-band">Ledger</header>
      <div className="deck-ledger__body" ref={host} />
    </section>
  );
}

type Deck = Pick<DeckState, "checks" | "needs" | "sel" | "view">;
/** What each of the tiles drawn with no Job open does, by id. Checks and Calls are plain filters; Map and Timeline are the stage's one switch. Their names, glyphs and order are `flight-controls.ts`'. */
const DECK_ACTS: Record<string, { pressed: (deck: Deck) => boolean; press: () => void }> = {
  checks: { pressed: (deck) => deck.checks, press: toggleChecks },
  needs: { pressed: (deck) => deck.needs, press: toggleNeeds },
  map: { pressed: (deck) => deck.sel === undefined && deck.view === "map", press: () => setView("map") },
  timeline: { pressed: (deck) => deck.sel === undefined && deck.view === "timeline", press: () => setView("timeline") },
};
const DECK_TIPS: Record<string, string> = { checks: "Checks: only what is running one" };

/**
 * The two filters and the view switch: one row of four equal tiles. **While a Session is open on the stage they are that
 * Session's** (`SessionTiles`), in the same block and under the same keys, and the four return when it closes.
 */
function FlightControl({ ship, props }: { ship: Ship; props: CockpitProps }) {
  const folded = useCollapse("controls");
  const deckTiles = tilesOf("deck", useFlightFile()).filter((one) => !one.hidden);
  const [editing, setEditing] = useState(false);
  /** The band's slot for Done and Cancel, which `FlightEdit` draws into. */
  const [acts, setActs] = useState<HTMLElement | null>(null);
  const checks = useDeck((deck) => deck.checks);
  const needs = useDeck((deck) => deck.needs);
  const sel = useDeck((deck) => deck.sel);
  const view = useDeck((deck) => deck.view);
  const deck: Deck = { checks, needs, sel, view };
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const open = sel?.kind === "session" ? ship.subjects.find((one) => one.key === sel.key && one.kind === "session") : undefined;
  const session = open === undefined ? undefined : sessions.find((one) => one.id === open.id);
  const served = props.state.holds.repositories ?? [];
  const found = session?.manifestId === undefined ? undefined : served.find((one) => jobIdOf(one) === session.manifestId);
  const repository = found === undefined ? undefined : { root: found.root, ...(found.manifest === undefined ? {} : { name: found.manifest.repository }) };
  const startIn = reportRoot(served, repository?.root);
  const clashes = useWriteClashes();
  const bays = session === undefined ? undefined : baysOf(session, props.state.held.state === "read" ? (props.state.held.held.slots ?? []) : [], clashes);
  const set: FlightSet = session !== undefined && draft !== undefined ? "job" : "deck";
  // Picking something else puts the other set under the tiles, and an edit of this one ends.
  useEffect(() => setEditing(false), [set]);
  return (
    <section className="deck-panel deck-controls" aria-label="Flight controls" tabIndex={-1} data-collapsed={folded.collapsed || undefined}>
      <header className="deck-band" {...folded.band}>
        Flight control
        <PanelKey id="controls" />
        {editing || folded.collapsed ? null : (
          <Button variant="ghost" size="sm" onClick={() => setEditing(true)}>
            Edit
          </Button>
        )}
        <span className="deck-band__acts" ref={setActs} />
      </header>
      {editing ? (
        <FlightEdit set={set} host={acts} onClose={() => setEditing(false)} />
      ) : session !== undefined && draft !== undefined ? (
        <SessionTiles session={session} draft={draft} repository={repository} bays={bays} onOpenLink={props.onOpenLink} {...(startIn === undefined ? {} : { startIn })} />
      ) : (
        <div className="deck-controls__block">
          {deckTiles.map((tile) => {
            const act = tile.builtIn === undefined ? undefined : DECK_ACTS[tile.id];
            const Glyph = flightGlyph(tile.glyph, "zap");
            const address = (tile.words ?? "").trim();
            return (
              <Tooltip key={tile.id} label={tile.builtIn === undefined ? `Open ${address}` : (DECK_TIPS[tile.id] ?? tile.label)} {...(tile.key === undefined ? {} : { shortcut: tile.key })} asChild>
                <button
                  type="button"
                  className="deck-tile"
                  aria-label={tile.label}
                  data-key={tile.key}
                  aria-pressed={act?.pressed(deck)}
                  onClick={() => (act === undefined ? isWebAddress(address) && props.onOpenLink(address) : act.press())}
                >
                  <Glyph size={20} aria-hidden="true" />
                  <span className="deck-tile__text">{tile.text}</span>
                  {tile.key === undefined ? null : (
                    <Kbd className="deck-tile__key" aria-hidden="true">
                      {tile.key}
                    </Kbd>
                  )}
                </button>
              </Tooltip>
            );
          })}
        </div>
      )}
    </section>
  );
}

/** Dispatch, between the controls and Autopilot: one tile that opens the dispatch sheet, where a person can see it and the key that does. */
function DispatchDeck() {
  const dispatching = useDispatching();
  if (dispatching === undefined) return null;
  return (
    <section className="deck-panel deck-dispatch" aria-label="Dispatch controls" tabIndex={-1}>
      <header className="deck-band">
        Dispatch
        <PanelKey id="dispatch" />
      </header>
      <div className="deck-dispatch__block">
        <Tooltip label="Dispatch" shortcut={keyFor("new_job")} asChild>
          <button type="button" className="deck-tile deck-dispatch__tile" aria-label="Dispatch" onClick={() => dispatching.openNew()}>
            <Send size={20} aria-hidden="true" />
            <span className="deck-tile__text">Dispatch</span>
            <Kbd className="deck-tile__key" aria-hidden="true">
              {keyFor("new_job")}
            </Kbd>
          </button>
        </Tooltip>
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
 * Autopilot as a compact card beside the controls: the switch is its title, and under it the repository, how it stands and a
 * thin bar. Switching it on gives the stage to its configuration; off, the stage is as it was.
 */
function AutopilotPanel({ props, autopilot }: { props: CockpitProps; autopilot: ReturnType<typeof useAutopilotLane> }) {
  const control = autopilot.control;
  const open = useDeck((deck) => deck.sel?.kind === "autopilot");
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
          {props.picked === null ? <RepositoryPicker props={props} autopilot={autopilot} /> : <span className="deck-autopilot__name">{props.picked.manifest?.repository ?? props.picked.root}</span>}
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

/** The lane on the stage: the real bar for the chosen repository, or the picker while none is. */
function AutopilotPane({ props, autopilot }: { props: CockpitProps; autopilot: ReturnType<typeof useAutopilotLane> }) {
  // The bar is the lane's own, drawn while its control has it open; opening this pane is what opens it.
  const control = autopilot.control;
  useEffect(() => {
    if (props.picked !== null && control !== undefined && !control.open) control.openNext();
  }, [props.picked, control]);
  return (
    <section className="deck-pane" aria-label="Autopilot lane">
      <PaneHead glyph={Plane} kind="Autopilot" title="Autopilot" />
      <div className="deck-pane__body">{props.picked === null ? <RepositoryPicker props={props} autopilot={autopilot} /> : autopilot.bar}
        {autopilot.sidings}
      </div>
    </section>
  );
}

// ---- The body ------------------------------------------------------------------------------------------

/** A clash mark anywhere on the deck opens the other party on the stage, where the window's own answer would leave the deck for a page. */
function ClashesOpenHere({ ship, children }: { ship: Ship; children: ReactNode }) {
  const written = useWriteClashes();
  const value = useMemo<Clashing>(
    () => ({
      of: clashingOf(written),
      open: (holder) => {
        const subject = ship.subjects.find((one) => one.kind === holder.kind && one.id === holder.id);
        if (subject === undefined) askToOpenHolder(holder);
        else select(selOfSubject(subject));
      },
    }),
    [written, ship.subjects],
  );
  return <ClashesProvider value={value}>{children}</ClashesProvider>;
}

/**
 * An Autopilot's strip, opened, brings the lane up for it once. A part of its own: the dispatching the window holds is a new object
 * each time the window draws, so a body that read it drew again every second.
 */
function AutopilotAsked({ control }: { control: ReturnType<typeof useAutopilotLane>["control"] }) {
  const dispatching = useDispatching();
  useEffect(() => {
    if (dispatching?.autopilotAsked !== true || control === undefined) return;
    control.openNext();
    enterAutopilot();
    dispatching.autopilotShown();
  });
  return null;
}

/** The replay's driver and its keys, apart from the body: they read the events by a five-second clock, and the body is handed the minute. */
function ReplayHost({ props, ship }: { props: CockpitProps; ship: Ship }) {
  const replaying = useDeck((deck) => deck.replay !== undefined);
  const events = useFleetEvents();
  const clock = useClock(CLOCK_MS);
  const replayable = useMemo(() => windowOf(events, clock)?.events ?? [], [events, clock]);
  const repositories = props.state.holds.repositories ?? [];
  const ownerOf = (number: number) => {
    const pr = ship.prs.find((one) => one.number === number);
    return pr?.owner === undefined ? undefined : `${pr.owner.kind}:${pr.owner.id}`;
  };
  const rootOf = (manifest: string | undefined) => repositories.find((one) => jobIdOf(one) === manifest)?.root;
  useReplayDriver(replayable, ownerOf, rootOf);
  useReplayKeys(replaying, replayable, ownerOf, rootOf, (target) => {
    const id = panelAt(target)?.id;
    return id === "stage" ? "map" : id;
  });
  return null;
}

/** **Drawn when what it is handed changes and not each second**: `CommandCentral` hands it the minute and callbacks that do not change. */
/** The containers `deck.css` takes the deck's margins and padding off. `:has(.deck)` on each restyled it whenever anything beneath it was inserted. */
const DECK_MARKS: readonly Mark[] = [
  { of: ".armada-shell__body", as: "data-deck" },
  { of: ".armada-shell__work", as: "data-deck" },
  { of: ".armada-screen__mounted", as: "data-deck" },
  { of: ".armada-screen__overview", as: "data-deck", parent: true },
];

export const DeckBody = memo(function DeckBody(props: CockpitProps) {
  const ship = useShip(props);
  const held = useDeck((deck) => deck.held);
  const visited = useDeck((deck) => deck.visited);
  const sel = useDeck((deck) => deck.sel);
  const autopilot = useAutopilotLane({ picked: props.picked, state: props.state, now: props.now });
  // A call an alert asked for is opened as a row of the list opens one, once this is on screen. `at` makes a second press land.
  const { bring, onBrought } = props;
  useEffect(() => {
    if (bring === undefined || bring === null) return;
    const item = ship.calls.find((one) => one.key === bring.key);
    if (item !== undefined) openCall(ship, item, props);
    onBrought?.();
    // `at` alone: the same call pressed twice is two landings.
  }, [bring?.at]);
  const banding = useMemo(() => bandingOf(props.picked, props.state.holds.repositories ?? []), [props.picked, props.state.holds.repositories]);
  const { listed, found } = useListed(ship, banding);
  // The selected Session's own calls are cards in its thread, so its row does not call him as well.
  const incoming = withoutOpenSession(incomingOf(ship.calls, held, visited, needingOf(ship)), sel);
  const [ledgerHost, setLedgerHost] = useState<HTMLElement | null>(null);
  const reading = sel?.kind === "session" && ship.subjects.some((one) => one.key === sel.key && one.kind === "session");
  // A Session with `work` draws its own panels in the places of Activity, Bays and Autopilot, and its status in the ring's.
  const sessions = useSessions();
  const unified = reading && sessions.some((one) => `session:${one.id}` === sel?.key && one.work !== undefined);
  const [sideHost, setSideHost] = useState<HTMLElement | null>(null);
  // A Session calling him pulses on its row; there is no panel of calls.
  const calling = useMemo(() => new Map(incoming.map((one) => [callOwnerOf(one), one] as const)), [incoming]);
  const hosts = useMemo<Hosts | undefined>(() => (unified ? { side: sideHost } : undefined), [unified, sideHost]);
  const selected = sel?.key;
  const grid = useRef<HTMLDivElement>(null);
  const deckMarks = useMarking(DECK_MARKS);
  const layout = unified ? "session" : reading ? "reading" : "idle";
  const { columns, resize, reset } = useColumns(grid, layout);
  const splits = useSplits(grid, layout);
  useActivityBeside(grid, layout);
  // While a Session that has built something is open, its ring stands above the ledger, where the Bays are otherwise.
  const builds = useBuilds(props.now);
  const building = reading && selected !== undefined ? builds.get(selected) : undefined;
  return (
    <ClashesOpenHere ship={ship}>
    {/* A replay plays on while the deck is drawn, from the events Bridge holds, and takes its own keys while it is up. */}
    <ReplayHost props={props} ship={ship} />
    <AutopilotAsked control={autopilot.control} />
    <div ref={deckMarks} className="armada-cockpit deck">
      <div ref={grid} className="deck-grid" style={columns === undefined ? undefined : ({ "--deck-left": `${columns.left}px`, "--deck-right": `${columns.right}px` } as CSSProperties)}>
        <div className="deck-left">
          <RepositoryScope props={props} />
          <List subjects={listed} calling={calling} banding={banding} selected={sel?.kind === "session" ? selected : undefined} builds={builds} picked={props.picked} found={found} onPick={(one) => openSubject(ship, one)} onEarlier={(id) => openLinked({ kind: "session", key: `session:${id}` })} onMove={(key, target, after) => moveRow(ship, key, target, after)} onAutopilot={(root) => (props.onPickRepository?.(root), enterAutopilot())} />
                    {columns === undefined ? null : <DockHandle quiet className="deck-handle" grows="right" label="the left column" width={columns.left} {...columns.range.left} onResize={(width) => resize("left", width)} onReset={() => reset("left")} />}
        </div>
        <div className="deck-centre">
          <div className="deck-stage">
            <Stage props={props} ship={ship} listed={listed} autopilot={autopilot} ledger={reading && !unified ? ledgerHost : null} hosts={hosts} />
            <MergeQueue props={props} ship={ship} />
          </div>
          <div className="deck-foot">
            <FlightControl ship={ship} props={props} />
            {unified ? null : <DispatchDeck />}
            {unified ? null : <AutopilotPanel props={props} autopilot={autopilot} />}
          </div>
        </div>
        <div className="deck-right">
          {columns === undefined ? null : <DockHandle quiet className="deck-handle" grows="left" label="the right column" width={columns.right} {...columns.range.right} onResize={(width) => resize("right", width)} onReset={() => reset("right")} />}
          {unified ? (
            <div className="sv-side" ref={setSideHost} />
          ) : reading ? (
            <>
              {building === undefined ? null : <BuildColumn key={building.key} build={building} />}
              <LedgerPanel host={setLedgerHost} />
            </>
          ) : <BaysPanel props={props} ship={ship} picked={sel?.kind === "session" ? selected : undefined} />}
          {unified ? null : <Activity props={props} listed={listed} ship={ship} />}
        </div>
        {splits}
      </div>
    </div>
    </ClashesOpenHere>
  );
});

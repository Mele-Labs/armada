// The Cockpit as a flight deck. A left column of the repository and every Job and Session with the feed under it; a stage
// that is the timeline until something is picked and then that thing (a Job's canvas, a Session's thread, a bay, a pull
// request, a call, Autopilot's lane); the merge queue under it; the bays and the incoming calls on the right; and along the
// foot the flight controls and Autopilot. Esc returns the stage to what it was. The state is held in `store.ts`, so the top bar,
// the body and the keys are one thing. The stage at rest is the map (the battlefield) or the timeline, one switch. Ported from
// proto/cockpit-flight-deck. #2301.

import { memo, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode, type RefObject } from "react";
import { Box, ChartNoAxesGantt, CircleDashed, FileCog, GitMerge, Link, LoaderCircle, Megaphone, Orbit, Pause, Plane, Play, Power, Send, ShieldCheck, SquareTerminal, Trash2, Unplug, UserCheck } from "lucide-react";
import { Badge, Button, ClashMark, ClashesProvider, DockHandle, DropdownMenu, Inspect, Kbd, Switch, Tooltip, keyFor, useMarking, type Clashing, type Mark } from "@armada/components";
import type { JobSummary, RepositorySummary, WorktreeSlot } from "@armada/protocol";
import { ADD, ALL, FIND_A_REPOSITORY, repositoryEntries, repositoryLabel, rootOf, useLayout, useRecentRepositories } from "@armada/shell";

import { age, useCursor, type Item } from "../../Dashboard";
import { clashingOf, useWriteClashes } from "../../clashes";
import { slotTroubleOf } from "../../session-view/slot-trouble";
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
import { useClock } from "./clock";
import { CLOCK_MS } from "./field";
import { ActivityGraph } from "../ActivityChart";
import { Timeline } from "../Timeline";
import type { CockpitProps } from "./props";
import { useShipActs } from "./acts";
import { Bays } from "./BayRows";
import { BuildColumn } from "./BuildColumn";
import { chipPreview } from "./chip-card";
import { CallCard, SubjectCard } from "./inspect-cards";
import { PHASE_GLYPH, PHASE_WORD, glyphOf, jobPhaseOf, wordOf, type BuildIn } from "./build";
import { useBuilds } from "./useBuilds";
import { DECK_LINK } from "./links";
import { holderKeyOf, useBays, slotKey } from "./bays";
import { useCollapse } from "./collapse";
import { PanelKey, panelAt } from "./keys";
import { queuedIn, queuedKey } from "./queued";
import type { QueuedIdea } from "@armada/screens/src/draft/sessions";
import { JobMission, MapView } from "./MapPane";
import { CallPane, JobPane, PaneHead, PrPane, SessionPane, SlotPane } from "./panes";
import { inOrder, useReplayDriver, useReplayKeys, windowOf } from "./replay";
import { RowPause } from "./RowPause";
import { SessionTiles } from "./SessionTiles";
import { reportRoot } from "./session-tiles";
import { checkoutSaid, heldOf } from "./YourCheckout";
import { forgeOf, quietSaid, ReadAgo } from "./forge";
import { useListed } from "./listed";
import type { Found } from "./find";
import { MainLight } from "./MainLight";
import { MarkGlyph } from "./shared";
import { toneOf } from "./order";
import { callOfSubject, callOwnerOf, enterAutopilot, incomingOf, keepOf, needingOf, leaveAutopilot, moveCursor, openCall, openSubject, select, selOfPr, selOfSubject, setFocus, setHover, setView, setQuery, setScope, setSearching, showMoreRows, toggleChecks, toggleNeeds, toggleOnly, useDeck, withoutOpenSession, SCOPES, type DeckState, type Scope } from "./store";
import { useShip, type Pr, type Ship, type Subject } from "./ship";
import type { Hosts } from "../../session-view/SessionWorkspace";
import "../cockpit.css";
import "./deck.css";


/** The slots the pool serves, narrowed to the picked repository as the ship's are. */
export const slotsOf = (props: Pick<CockpitProps, "state" | "picked">): readonly WorktreeSlot[] =>
  props.state.held.state === "read" ? (props.state.held.held.slots ?? []).filter((one) => props.picked?.manifest === undefined || one.manifest_id === props.picked.manifest.id) : [];

const repoNameOf = (props: Pick<CockpitProps, "state">, manifest: string) => props.state.holds.repositories?.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest;

// ---- The left column ----------------------------------------------------------------------------------

/** The repository, at the head of the left column: it narrows the list, the feed, the bays and the queue, as the title row's picker does. */
function RepositoryScope({ props }: { props: CockpitProps }) {
  const [recent] = useRecentRepositories();
  const repositories = props.state.holds.repositories ?? [];
  const pick = props.onPickRepository;
  const add = props.onAddRepository;
  if (props.onGoTo === undefined && (repositories.length === 0 || pick === undefined)) return null;
  return (
    <div className="deck-scope">
      {repositories.length === 0 || pick === undefined ? null : (
        <DropdownMenu
          align="start"
          portal
          triggerLabel={props.picked === null ? "All repositories" : repositoryLabel(props.picked, repositories)}
          triggerAriaLabel="Repository"
          searchLabel={FIND_A_REPOSITORY}
          entries={repositoryEntries(repositories, { scope: props.picked?.root ?? null, all: "All repositories", add: add !== undefined, recent })}
          onSelect={(chosen) => {
            if (chosen === ADD) return add?.();
            const root = rootOf(chosen);
            pick(root === ALL ? null : root);
          }}
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

/** Where a Session or Job is in its build, as the one glyph of its phase. A Session with no build and a Job with no plan have an empty cell. */
function Phase({ subject, builds, jobs }: { subject: Subject; builds: ReadonlyMap<string, BuildIn>; jobs: readonly JobSummary[] }) {
  const build = subject.kind === "session" ? builds.get(subject.key) : undefined;
  const phase = subject.kind === "job" ? jobPhaseOf(jobs.find((one) => one.id === subject.id)?.tasks) : undefined;
  const Glyph = build !== undefined ? glyphOf(build) : phase === undefined ? undefined : PHASE_GLYPH[phase];
  const word = build !== undefined ? wordOf(build) : phase === undefined ? undefined : PHASE_WORD[phase];
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
    <li className="deck-list__more" ref={mark} role="status" aria-label="Loading more Sessions">
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
      searchLabel="Search Work"
      search={{ value: typed, onChange: setQuery, onEnter: () => rows.current?.querySelector<HTMLElement>(".deck-row")?.focus() }}
      open={open}
      onOpenChange={setSearching}
      entries={SCOPES.map(({ id, label }) => ({ kind: "item" as const, id, label, selected: id === scope }))}
      onSelect={(id) => setScope(id as Scope)}
    />
  );
}

/** What each thing a Session waits on is called, by the item's source. */
const CALL_WORD: Record<string, string> = { ask_card: "Question", window_ask: "Question", permission: "Permission", plan: "Plan", judge: "Judge concern", slot_choice: "Slot", walk: "Walk", done: "Done" };

/** What a call says, as the tooltip on its row: what it is, then its words. */
const callLabel = (call: Item): string => {
  const kind = call.kind.replace(/^Session ?/, "");
  const word = CALL_WORD[call.waiting?.item.source ?? ""] ?? (kind === "" ? "Waiting" : kind.charAt(0).toUpperCase() + kind.slice(1));
  return call.fact === "" ? word : `${word}: ${call.fact}`;
};

function List({ subjects, calling, banding, selected, builds, jobs, picked, found, onPick }: { subjects: readonly Subject[]; calling: ReadonlyMap<string, Item> | undefined; banding: readonly Banding[] | undefined; selected: string | undefined; builds: ReadonlyMap<string, BuildIn>; jobs: readonly JobSummary[]; picked: RepositorySummary | null; found: ReadonlyMap<string, Found> | undefined; onPick: (subject: Subject) => void }) {
  const folded = useCollapse("list");
  const cursor = useDeck((deck) => deck.cursor);
  const hover = useDeck((deck) => deck.hover);
  const limit = useDeck((deck) => deck.limit);
  const ideas = useQueued();
  const panel = useRef<HTMLElement>(null);
  const drawn = subjects.slice(0, limit);
  const row = (one: Subject) => {
    const Kind = one.kind === "job" ? Box : one.linked !== undefined ? Link : SquareTerminal;
    const kindSaid = one.kind === "job" ? "Job" : one.linked !== undefined ? (one.linked.group === undefined ? "Linked Session" : `Linked Session, handed from ${one.linked.group}`) : "Session";
    const quiet = one.since ?? one.step?.label ?? "";
    const said = quiet.length > 9 ? "" : quiet;
    // What a search matched in the Session, in place of its age.
    const hit = found?.get(one.key);
    // A Session calling him: its row pulses a bar, and its tooltip names the call.
    const call = calling?.get(one.key);
    return (
      <li key={one.key} className="deck-list__item">
        <Inspect card={<SubjectCard one={one} {...(call === undefined ? {} : { calling: callLabel(call) })} />} name={one.title} side="beside" pin={false}>
        <button type="button" className="deck-row" data-row={one.key} data-calling={call === undefined ? undefined : ""} data-linked={one.linked !== undefined || undefined} data-job-id={one.kind === "job" ? one.id : undefined} data-tone={toneOf(one)} data-past={one.past} data-selected={selected === one.key || undefined} data-cursor={cursor === one.key || undefined} data-hover={hover === one.key || undefined} aria-label={one.title} aria-current={selected === one.key || undefined} onClick={() => onPick(one)} onFocus={() => (moveCursor(one.key), setHover(one.key))} onBlur={() => setHover(undefined)} onMouseEnter={() => setHover(one.key)} onMouseLeave={() => setHover(undefined)}>
          <Tooltip label={kindSaid}>
            <span className="deck-row__kind" role="img" aria-label={kindSaid}>
              <Kind size={14} aria-hidden="true" />
            </span>
          </Tooltip>
          <span className="deck-row__title">{one.title}</span>
          {hit === undefined || hit.short === "" ? (
            <span className="deck-row__said">{said}</span>
          ) : (
            <Tooltip label={hit.whole}>
              <span className="deck-row__said" data-found="" role="img" aria-label={hit.whole}>
                {hit.short}
              </span>
            </Tooltip>
          )}
          <Phase subject={one} builds={builds} jobs={jobs} />
          <MarkGlyph mark={{ ...one.mark, tone: toneOf(one) }} />
        </button>
        </Inspect>
        {one.kind === "pr" ? null : <ClashMark kind={one.kind} id={one.id} />}
        {one.kind === "session" ? <RowPause id={one.id} /> : null}
      </li>
    );
  };
  return (
    // A press on the list and on no row lets the cursor go.
    <section ref={panel} className="deck-panel deck-list" aria-label="Work" tabIndex={-1} data-collapsed={folded.collapsed || undefined} onClick={(event) => (event.target as Element).closest(".deck-row") === null && moveCursor(undefined)}>
      <header className="deck-band" {...folded.band}>
        Work
        <Filter rows={panel} />
        <PanelKey id="list" />
      </header>
      <ul className="deck-list__rows">
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
    </section>
  );
}

/**
 * Activity and Incoming calls stand as tall as the merge queue and the flight controls beside them: from the queue's top to the foot's
 * bottom. Each is in a side column, so the height is set on the grid and both read it. A size he has dragged to takes over from it.
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
  const only = useDeck((deck) => deck.only);
  const needs = useDeck((deck) => deck.needs);
  const checks = useDeck((deck) => deck.checks);
  const sel = useDeck((deck) => deck.sel);
  const focus = useDeck((deck) => deck.focus);
  const replay = useDeck((deck) => deck.replay);
  const views = useMemo(() => viewsOf(props.state), [props.state]);
  const scope = useDeck((deck) => deck.scope);
  const query = useDeck((deck) => deck.query);
  const keep = useMemo(() => keepOf(listed, { only, needs, checks, scope, query }), [listed, only, needs, checks, scope, query]);
  // The event held lit: the one picked, or the one the playhead has last passed. Its column is marked.
  const events = useFleetEvents();
  // A Job or Session open on the stage is the only one the graph tells of.
  const open = sel?.kind === "job" || sel?.kind === "session" ? ship.subjects.find((one) => one.key === sel.key && one.kind === sel.kind) : undefined;
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
        jobs={props.state.jobs}
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
  const only = useDeck((deck) => deck.only);
  const needs = useDeck((deck) => deck.needs);
  const checks = useDeck((deck) => deck.checks);
  const scope = useDeck((deck) => deck.scope);
  const query = useDeck((deck) => deck.query);
  const keep = useMemo(() => keepOf(listed, { only, needs, checks, scope, query }), [listed, only, needs, checks, scope, query]);
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

/** The bays, as views: the pool's own states and who is behind each, with the holders the slot events name. */
function useBayViews(props: CockpitProps) {
  const events = useFleetEvents();
  const slots = slotsOf(props);
  return useBays(slots, events, undefined, props.now);
}

function Stage({ props, ship, listed, autopilot, ledger, hosts }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; autopilot: ReturnType<typeof useAutopilotLane>; ledger: HTMLElement | null; hosts: Hosts | undefined }) {
  const sel = useDeck((deck) => deck.sel);
  const held = useDeck((deck) => deck.held);
  const view = useDeck((deck) => deck.view);
  const acts = useShipActs(props, { onMergeJob: props.onMergeJob });
  const subject = sel === undefined ? undefined : ship.subjects.find((one) => one.key === sel.key && one.kind !== "pr");
  const slot = sel?.kind === "slot" ? slotsOf(props).find((one) => slotKey(one) === sel.key) : undefined;
  const bays = useBayViews(props);
  const pr = sel?.kind === "pr" ? ship.prs.find((one) => one.key === sel.key) : undefined;
  const call = sel?.kind === "call" ? ship.calls.find((one) => one.key === sel.key) : undefined;
  // What was picked is gone (a Job ended, a call was answered elsewhere): back to the timeline.
  const lane = sel?.kind === "autopilot";
  const gone = sel !== undefined && !lane && subject === undefined && slot === undefined && pr === undefined && call === undefined;
  useEffect(() => {
    if (gone) select(undefined);
  }, [gone]);
  const onPr = (one: Pr) => select(selOfPr(one));
  // A move makes its Session a moment before it opens it, so the ship is read as it stands then and not as it stood when pressed.
  const latest = useRef(ship);
  latest.current = ship;
  const owed = subject === undefined ? undefined : callOfSubject(subject, held);
  const asked = owed === undefined ? undefined : ship.calls.find((one) => one.key === owed);
  if (sel?.kind === "job" && subject !== undefined) return <JobPane subject={subject} props={props} acts={acts} onPr={onPr} call={asked} mission={(run) => <JobMission props={props} ship={ship} listed={listed} subject={subject} run={run} />} />;
  // A Session this ship does not hold opens on its own page, as a link to one does.
  const openSession = (id: string) => {
    const held = latest.current.subjects.find((one) => one.kind === "session" && one.id === id);
    if (held === undefined) props.onOpenSession(id);
    else openSubject(latest.current, held);
  };
  if (sel?.kind === "session" && subject !== undefined) return <SessionPane subject={subject} props={props} acts={acts} ledger={ledger} hosts={hosts} openSession={openSession} />;
  const bay = slot === undefined ? undefined : bays.find((one) => one.view.key === sel?.key);
  if (slot !== undefined && bay !== undefined) return <SlotPane view={bay.view} ledger={bay.ledger} clock={props.now} repo={repoNameOf(props, slot.manifest_id)} known={new Set(ship.subjects.map((one) => one.key))} props={props} />;
  if (pr !== undefined) return <PrPane pr={pr} owner={ship.subjects.find((one) => one.pr?.key === pr.key)} props={props} acts={acts} />;
  if (call !== undefined) return <CallPane item={call} props={props} />;
  if (lane) return <AutopilotPane props={props} autopilot={autopilot} />;
  if (view === "map") return <MapView props={props} ship={ship} listed={listed} />;
  return <TimelineView props={props} ship={ship} listed={listed} />;
}

// ---- The merge queue ----------------------------------------------------------------------------------

/** The repository a Job, a Session, a pull request or a bay open on the stage belongs to, by the root its line is served under. */
function queueRootOf(sel: DeckState["sel"], ship: Ship, repositories: readonly RepositorySummary[]): string | undefined {
  if (sel?.kind === "slot") {
    const manifest = sel.key.slice("slot:".length, sel.key.lastIndexOf("/"));
    return repositories.find((one) => one.manifest?.id === manifest)?.root;
  }
  if (sel?.kind === "pr") return ship.prs.find((one) => one.key === sel.key)?.root;
  if (sel?.kind === "job" || sel?.kind === "session") return ship.subjects.find((one) => one.key === sel.key && one.kind === sel.kind)?.root;
  return undefined;
}

/**
 * The merge queue: one row across the repositories, main first and then one chip for each pull request, each repository's
 * nearest main first. Main is one light, green while every main is and red if any is; a red one opens a card with the Merge line's
 * panel for each red (`MainLight`), and a mark on it with the checkout section while a served checkout is held behind main. A press on a chip opens the pull request on the stage. No count.
 */
function MergeQueue({ props, ship, selected }: { props: CockpitProps; ship: Ship; selected: string | undefined }) {
  const sessions = useSessions();
  const sel = useDeck((deck) => deck.sel);
  const served = viewsOf(props.state);
  // A layout that hides the merge line's panel hides this queue, its one counterpart.
  const shown = useLayout("dashboard.panels").shown.some((one) => one.id === "merge-line");
  if (served.length === 0 || !shown) return null;
  // On All with several repositories served, the queue is the open Job's, Session's or pull request's repository, and nobody's with none.
  const wide = props.picked === null && served.length > 1;
  const root = wide ? queueRootOf(sel, ship, props.state.holds.repositories ?? []) : undefined;
  const views = wide ? served.filter((view) => view.root === root) : served;
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
  if (views.length === 0) {
    return (
      <section className="deck-panel deck-queue" data-dim aria-label="Merge queue" tabIndex={-1}>
        <div className="deck-queue__row">
          {merge}
          <span className="deck-queue__say">Select a repo to see PRs</span>
          <PanelKey id="queue" />
        </div>
      </section>
    );
  }
  const reds = views.flatMap((view) =>
    view.hub?.main?.state !== "red" ? [] : [{ view: { ...view, hub: view.hub }, name: ship.repos.find((one) => one.root === view.root)?.name ?? view.root }],
  );
  const held = heldOf(views, (root) => ship.repos.find((one) => one.root === root)?.name ?? root);
  const red = reds.map((one) => one.name);
  const forge = forgeOf(views, (root) => ship.repos.find((one) => one.root === root)?.name ?? root);
  const quiet = quietSaid(forge, views.length > 1);
  const mainSaid = `${red.length > 0 ? `Main is red in ${red.join(", ")}` : "Main is green"}${held.length > 0 ? `, ${checkoutSaid(held).replace(/^Checkout/, "checkout")}` : ""}`;
  // While a Session is open its pull requests are framed in the line; with none open, or a Job or a pull request, none are.
  const viewed = sel?.kind === "session" ? sessions.find((one) => `session:${one.id}` === sel.key) : undefined;
  const chipOf = (view: (typeof views)[number], dot: ReturnType<typeof dotsOf>[number]) => {
    const pr = ship.prs.find((one) => one.root === view.root && `pull:${one.number}` === dot.key);
    const related = viewed !== undefined && pr !== undefined && sessionHolds(viewed, pr);
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
            <button type="button" className="deck-chip" data-selected={selected === pr.key || undefined} data-related={related || undefined} aria-label={dot.tip} onClick={() => select(selOfPr(pr))}>
              {inside}
            </button>
          )}
        </Inspect>
      </li>
    );
  };
  // Each repository's queue first, in its order and framed as the queue, then its other pull requests past a rule.
  const marks = views.flatMap((view, at) => {
    const dots = dotsOf(view, props.state.mergeLines, views, sessions);
    const queued = dots.filter((dot) => dot.queued).sort((a, b) => (a.card.place ?? Infinity) - (b.card.place ?? Infinity));
    const open = dots.filter((dot) => !dot.queued);
    return [
      ...(at === 0 ? [] : [<li key={`gap:${view.root}`} className="deck-queue__gap" aria-hidden="true" />]),
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
  });
  return (
    <section className="deck-panel deck-queue" aria-label="Merge queue" tabIndex={-1}>
      <div className="deck-queue__row">
        {merge}
        <ol className="deck-chips" onWheel={(event) => {
          // A wheel turned over the line moves it along, since it scrolls only across.
          if (event.deltaY !== 0 && event.deltaX === 0) event.currentTarget.scrollLeft += event.deltaY;
        }}>
          <li>
            {reds.length > 0 || held.length > 0 ? (
              <MainLight props={props} ship={ship} reds={reds} held={held} said={mainSaid} forge={forge} quiet={quiet} />
            ) : (
              <Tooltip label={forge.readAt === undefined ? mainSaid : <>{mainSaid}. <ReadAgo at={forge.readAt} /></>} asChild>
                <span className="deck-chip" data-main role="img" aria-label={mainSaid}>
                  <span className="deck-chip__dot" data-tone="calm" data-stale={quiet !== undefined || undefined} aria-hidden="true" />
                  <span>main</span>
                  {quiet === undefined ? null : (
                    <Tooltip label={quiet}>
                      <span className="deck-chip__quiet" role="img" aria-label={quiet}>
                        <Unplug size={12} aria-hidden="true" />
                      </span>
                    </Tooltip>
                  )}
                </span>
              </Tooltip>
            )}
          </li>
          {marks}
        </ol>
        <PanelKey id="queue" />
      </div>
    </section>
  );
}

// ---- The right column, and the foot -------------------------------------------------------------------

/** A Job or Session picked in the list narrows the bays to those it holds, or held last where stranded. Nothing picked, every bay. */
function BaysPanel({ props, ship, selected, picked }: { props: CockpitProps; ship: Ship; selected: string | undefined; picked: string | undefined }) {
  const bays = useBayViews(props);
  const working = useMemo(() => new Set(ship.subjects.filter((one) => one.phase === "working").map((one) => one.key)), [ship.subjects]);
  const shown = bays.map((one) => one.view).filter((view) => {
    if (picked === undefined) return true;
    return holderKeyOf(view) === picked;
  });
  const hovered = useDeck((deck) => deck.hover);
  const change = props.bayActs?.onChange;
  const addSlot = change === undefined ? undefined : (manifest: string) => change(manifest, { act: "add" });
  return <Bays views={shown} hovered={hovered} onHover={setHover} repoName={(manifest) => repoNameOf(props, manifest)} working={working} selected={selected} onSelect={(key) => select({ kind: "slot", key })} onOpenSlots={props.onGoTo === undefined ? undefined : () => props.onGoTo?.(DECK_LINK.bays)} onAddSlot={addSlot} />;
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

type Deck = Pick<DeckState, "only" | "checks" | "needs" | "sel" | "view">;
type Tile = { label: string; tip?: string; text: string; Glyph: typeof Box; pressed: (deck: Deck) => boolean; press: () => void };

/** The six tiles in the order their keys, `1` to `6`, press them. Jobs only, Sessions only, Checks and Calls are plain filters; Map and Timeline are the stage's one switch. */
export const TILES: readonly Tile[] = [
  { label: "Jobs only", text: "Jobs", Glyph: Box, pressed: (deck) => deck.only === "jobs", press: () => toggleOnly("jobs") },
  { label: "Sessions only", text: "Sessions", Glyph: SquareTerminal, pressed: (deck) => deck.only === "sessions", press: () => toggleOnly("sessions") },
  { label: "Checks", tip: "Checks: only what is running one", text: "Checks", Glyph: ShieldCheck, pressed: (deck) => deck.checks, press: toggleChecks },
  { label: "Needs me", text: "Calls", Glyph: Megaphone, pressed: (deck) => deck.needs, press: toggleNeeds },
  { label: "Map", text: "Map", Glyph: Orbit, pressed: (deck) => deck.sel === undefined && deck.view === "map", press: () => setView("map") },
  { label: "Timeline", text: "Timeline", Glyph: ChartNoAxesGantt, pressed: (deck) => deck.sel === undefined && deck.view === "timeline", press: () => setView("timeline") },
];

/**
 * The four filters and the view switch: one row of six equal tiles. **While a Session is open on the stage they are that
 * Session's** (`SessionTiles`), in the same block and under the same keys, and the six return when it closes.
 */
function FlightControl({ ship, props }: { ship: Ship; props: CockpitProps }) {
  const folded = useCollapse("controls");
  const only = useDeck((deck) => deck.only);
  const checks = useDeck((deck) => deck.checks);
  const needs = useDeck((deck) => deck.needs);
  const sel = useDeck((deck) => deck.sel);
  const view = useDeck((deck) => deck.view);
  const deck: Deck = { only, checks, needs, sel, view };
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const open = sel?.kind === "session" ? ship.subjects.find((one) => one.key === sel.key && one.kind === "session") : undefined;
  const session = open === undefined ? undefined : sessions.find((one) => one.id === open.id);
  const served = props.state.holds.repositories ?? [];
  const found = session?.manifestId === undefined ? undefined : served.find((one) => one.manifest?.id === session.manifestId);
  const repository = found === undefined ? undefined : { root: found.root, ...(found.manifest === undefined ? {} : { name: found.manifest.repository }) };
  const startIn = reportRoot(served, repository?.root);
  const clashes = useWriteClashes();
  const trouble = session === undefined ? undefined : slotTroubleOf(session, props.state.held.state === "read" ? (props.state.held.held.slots ?? []) : [], clashes);
  return (
    <section className="deck-panel deck-controls" aria-label="Flight controls" tabIndex={-1} data-collapsed={folded.collapsed || undefined}>
      <header className="deck-band" {...folded.band}>
        Flight control
        <PanelKey id="controls" />
      </header>
      {session !== undefined && draft !== undefined ? (
        <SessionTiles session={session} draft={draft} repository={repository} trouble={trouble} {...(startIn === undefined ? {} : { startIn })} />
      ) : (
        <div className="deck-controls__block">
          {TILES.map((tile, at) => (
            <Tooltip key={tile.label} label={tile.tip ?? tile.label} shortcut={String(at + 1)} asChild>
              <button type="button" className="deck-tile" aria-label={tile.label} aria-pressed={tile.pressed(deck)} onClick={tile.press}>
                <tile.Glyph size={20} aria-hidden="true" />
                <span className="deck-tile__text">{tile.text}</span>
                <Kbd className="deck-tile__key" aria-hidden="true">
                  {at + 1}
                </Kbd>
              </button>
            </Tooltip>
          ))}
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

function Incoming({ calls, now, ship, hosts }: { calls: readonly Item[]; now: number; ship: Ship; hosts: Pick<CockpitProps, "onOpen" | "onOpenSession"> }) {
  const folded = useCollapse("calls");
  return (
    <section className="deck-panel deck-incoming" aria-label="Incoming calls" tabIndex={-1} data-collapsed={folded.collapsed || undefined}>
      <header className="deck-band" {...folded.band}>
        Incoming calls
        <PanelKey id="calls" />
      </header>
      <ul className="deck-incoming__cards">
        {calls.map((item, at) => {
          const Icon = item.icon;
          return (
            <li key={item.key}>
              <Inspect card={<CallCard item={item} now={now} />} name={item.title} side="beside" pin={false}>
              <button type="button" className="deck-hail" data-hue={item.hue} data-first={at === 0 || undefined} aria-label={`${item.kind}: ${item.title}`} onClick={() => openCall(ship, item, hosts)}>
                <Tooltip label={item.kind}>
                  <span className="deck-hail__kind" role="img" aria-label={item.kind}>
                    <Icon size={16} aria-hidden="true" />
                  </span>
                </Tooltip>
                <span className="deck-hail__title">{item.title}</span>
                <span className="deck-hail__age">{age(item.at, now)}</span>
              </button>
              </Inspect>
            </li>
          );
        })}
      </ul>
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

/** Helm's footer reads the Job the cursor is on. Apart from the body, which the cursor's every move would draw. */
function CursorJob({ listed, jobs, onCursor }: { listed: readonly Subject[]; jobs: CockpitProps["state"]["jobs"]; onCursor: CockpitProps["onCursor"] }) {
  const cursor = useDeck((deck) => deck.cursor);
  const job = useMemo(() => {
    const at = listed.find((one) => one.key === cursor);
    return at?.kind === "job" ? jobs.find((one) => one.id === at.id) : undefined;
  }, [listed, cursor, jobs]);
  useCursor(job, onCursor);
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
  const rootOf = (manifest: string | undefined) => repositories.find((one) => one.manifest?.id === manifest)?.root;
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
  const { listed, found } = useListed(ship, props.state.jobs, banding);
  // The selected Session's own calls are cards in its thread, so Incoming calls leaves them out and nothing shows twice.
  const incoming = withoutOpenSession(incomingOf(ship.calls, held, visited, needingOf(ship)), sel);
  const [ledgerHost, setLedgerHost] = useState<HTMLElement | null>(null);
  const reading = sel?.kind === "session" && ship.subjects.some((one) => one.key === sel.key && one.kind === "session");
  // A Session with `work` draws its own panels in the places of Activity, Bays and Autopilot, and its status in the ring's.
  const sessions = useSessions();
  const unified = reading && sessions.some((one) => `session:${one.id}` === sel?.key && one.work !== undefined);
  const [sideHost, setSideHost] = useState<HTMLElement | null>(null);
  // With a Session open its calling neighbours pulse on their rows, in place of a panel of calls.
  const calling = useMemo(() => (unified ? new Map(incoming.map((one) => [callOwnerOf(one), one] as const)) : undefined), [unified, incoming]);
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
    <CursorJob listed={listed} jobs={props.state.jobs} onCursor={props.onCursor} />
    <AutopilotAsked control={autopilot.control} />
    <div ref={deckMarks} className="armada-cockpit deck">
      <div ref={grid} className="deck-grid" style={columns === undefined ? undefined : ({ "--deck-left": `${columns.left}px`, "--deck-right": `${columns.right}px` } as CSSProperties)}>
        <div className="deck-left">
          <RepositoryScope props={props} />
          <List subjects={listed} calling={calling} banding={banding} selected={sel?.kind === "job" || sel?.kind === "session" ? selected : undefined} builds={builds} jobs={props.state.jobs} picked={props.picked} found={found} onPick={(one) => openSubject(ship, one)} />
          {unified ? null : <Incoming calls={incoming} now={props.now} ship={ship} hosts={props} />}
          {columns === undefined ? null : <DockHandle quiet className="deck-handle" grows="right" label="the left column" width={columns.left} {...columns.range.left} onResize={(width) => resize("left", width)} onReset={() => reset("left")} />}
        </div>
        <div className="deck-centre">
          <div className="deck-stage">
            <Stage props={props} ship={ship} listed={listed} autopilot={autopilot} ledger={reading && !unified ? ledgerHost : null} hosts={hosts} />
            <MergeQueue props={props} ship={ship} selected={sel?.kind === "pr" ? selected : undefined} />
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
          ) : <BaysPanel props={props} ship={ship} selected={sel?.kind === "slot" ? selected : undefined} picked={sel?.kind === "job" || sel?.kind === "session" ? selected : undefined} />}
          {unified ? null : <Activity props={props} listed={listed} ship={ship} />}
        </div>
        {splits}
      </div>
    </div>
    </ClashesOpenHere>
  );
});

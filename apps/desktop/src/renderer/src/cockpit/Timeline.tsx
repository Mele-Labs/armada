// The Cockpit's other face: what Fleet did, newest first, one row per event, each naming what it happened
// to. Three takes of it share one set of marks, Lanes first and his last pick kept: a feed of every
// event, a lane for each Job and Session with its events along the last two hours,
// and the lanes beside the feed. A pull request is no lane: its events are marks on the track of the Job
// or Session that owns it, and one nobody owns is left out. The filter above narrows it as it narrows the
// grid. What it draws is Fleet's timeline (`fleet-events.tsx`). Nothing here moves but a subject that is working.

import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { BellRing, Box, ChevronDown, ChevronRight, CircleCheck, CircleDashed, CircleX, Clock, FilePenLine, GitMerge, LoaderCircle, Power, Reply, ScrollText, Split as SplitLine, SquareTerminal, Terminal } from "lucide-react";
import { Button, Tabs, Tooltip } from "@armada/components";
import type { DashboardTab } from "@armada/overview";
import type { JobSummary, TimelineKind } from "@armada/protocol";
import type { MergeLineView } from "@armada/screens";
import { titleOf as plainTitle } from "@armada/screens";

import { useSessions } from "../sessions-draft";
import { stateOf } from "../session-state";
import { timelineOwner } from "./owner";
import { QUIET_KINDS, STANDINGS, ageOf, fromWire, keyOf, lanesFor, lanesOf, titleOf, useFleetEvents, type EventTone, type FleetEvent, type Lane, type Standing, type Subject } from "./fleet-events";
import "./timeline.css";

type Take = "feed" | "lanes" | "split";

const TAKE_KEY = "armada.bridge.cockpit-timeline-take";

/** Lanes until he picks another, and the pick kept in the window's own storage as the grid or timeline choice is. */
function useTake(): [Take, (take: Take) => void] {
  const [take, set] = useState<Take>(() => {
    try {
      const kept = window.localStorage.getItem(TAKE_KEY);
      return kept === "feed" || kept === "split" ? kept : "lanes";
    } catch {
      return "lanes";
    }
  });
  return [
    take,
    (next) => {
      set(next);
      try {
        window.localStorage.setItem(TAKE_KEY, next);
      } catch {
        // Unremembered, which is the honest answer for a preference.
      }
    },
  ];
}

const TAKES = [
  { id: "feed", label: "Feed", hint: "One row for each event, newest first" },
  { id: "lanes", label: "Lanes", hint: "A row for each Job and Session, its events along the last two hours" },
  { id: "split", label: "Split", hint: "Each subject, and its events beside it" },
];

const TONE: Record<EventTone, string> = {
  run: "var(--status-running)",
  ask: "var(--status-awaiting-review)",
  bad: "var(--status-completed-failed)",
  ok: "var(--status-completed-success)",
  idle: "var(--status-not-started)",
};

const LEGEND: readonly (readonly [EventTone, string])[] = [
  ["run", "working"],
  ["ask", "needs you"],
  ["bad", "failing"],
  ["ok", "passed"],
  ["idle", "waiting"],
];

/** How far back the lanes reach, and how much of it the glass shows at once: the rest is a scroll to the left. */
const MINUTES = 120;
const SHOWN = 60;
const WINDOW = MINUTES * 60_000;

const KIND: Record<Subject["kind"], string> = { job: "Job", session: "Session", main: "Main" };
const GLYPH = { job: Box, session: SquareTerminal, main: GitMerge };

/** What an event's mark is shaped as: a pull request's keeps its own shape on its owner's track. */
const shapeOf = (event: FleetEvent): string => (event.pull === undefined ? event.subject.kind : "pull_request");

const toneOf = (tone: EventTone) => ({ ["--tone" as string]: TONE[tone] }) as CSSProperties;
const clock = (at: string) => new Date(at).toLocaleTimeString([], { hour12: false });

/** Where a subject opens: the Job or the Session. Main opens nothing. */
type Open = (subject: Subject) => void;

function Glyph({ subject }: { subject: Subject }) {
  const Icon = GLYPH[subject.kind];
  return (
    <Tooltip label={KIND[subject.kind]}>
      <span className="armada-tl__glyph">
        <Icon size={12} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** A subject's name, as a press that opens it where it opens anything. */
function Who({ subject, open, children, glyph = true }: { subject: Subject; open: Open; children?: ReactNode; /** The kind's glyph beside the name. A stacked row draws it in its own column. */ glyph?: boolean }) {
  const inside = (
    <>
      {glyph ? <Glyph subject={subject} /> : null}
      <span className="armada-tl__title">{titleOf(subject)}</span>
      {children}
    </>
  );
  return subject.kind === "main" ? (
    <span className="armada-tl__who">{inside}</span>
  ) : (
    <button type="button" className="armada-tl__who" data-opens aria-label={`${KIND[subject.kind]}: ${titleOf(subject)}`} onClick={() => open(subject)}>
      {inside}
    </button>
  );
}

const STANDING_MARK: Record<Standing, { icon: typeof Clock; said: string }> = {
  ask: { icon: BellRing, said: "Needs you" },
  bad: { icon: CircleX, said: "Failing" },
  stuck: { icon: Clock, said: "Quiet" },
  run: { icon: LoaderCircle, said: "Working" },
  idle: { icon: CircleDashed, said: "Waiting" },
  done: { icon: CircleCheck, said: "Finished" },
};

/** How a subject stands now: the one live mark, which turns only while it is working. */
function Stand({ lane, now }: { lane: Lane; now: number }) {
  // A Session that is done is in Needs you with its own mark, and one with an unread reply wears its own, the ones the Sessions list wears.
  const { icon: Icon, said } = lane.session === "replied" ? { icon: Reply, said: "Unread reply" } : lane.session === "done" ? { icon: Power, said: "Done" } : STANDING_MARK[lane.standing];
  const failed = lane.standing === "done" && lane.head.tone === "bad";
  const Shown = failed ? CircleX : Icon;
  const tone: EventTone = failed ? "bad" : lane.standing === "run" ? "run" : lane.standing === "ask" ? "ask" : lane.standing === "bad" ? "bad" : lane.standing === "done" ? "ok" : "idle";
  const label = lane.standing === "stuck" ? `Quiet for ${ageOf(lane.head.at, now)}` : failed ? "Failed" : said;
  return (
    <Tooltip label={label}>
      <span className="armada-tl__standing" role="img" aria-label={label} data-standing={lane.standing} style={toneOf(tone)}>
        <Shown size={12} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

function Age({ event, now }: { event: FleetEvent; now: number }) {
  return (
    <Tooltip label={clock(event.at)}>
      <time className="armada-tl__age" dateTime={event.at}>
        {ageOf(event.at, now)}
      </time>
    </Tooltip>
  );
}

/** A mark at the end of a stacked row's line, only where the event is owed to him, failing, or the subject has gone quiet: a normal event has none. */
function Flag({ event, standing }: { event: FleetEvent; standing: Standing | undefined }) {
  const mark = event.tone === "ask" ? STANDING_MARK.ask : event.tone === "bad" ? STANDING_MARK.bad : standing === "stuck" ? STANDING_MARK.stuck : undefined;
  if (mark === undefined) return null;
  const Icon = mark.icon;
  return (
    <Tooltip label={mark.said}>
      <span className="armada-tl__flag" role="img" aria-label={mark.said}>
        <Icon size={14} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

const bucketOf = (event: FleetEvent, now: number): string => {
  const minutes = (now - Date.parse(event.at)) / 60_000;
  return minutes < 3 ? "Just now" : minutes < 15 ? "Last 15 minutes" : minutes < 60 ? "Last hour" : "Earlier";
};

/**
 * The kinds that wear a glyph in the spine where the rest wear a dot: the coalesced work of a Session or a
 * Job's Drone, and a Session's subagents. A waiting, a resolution and a Session started by another keep the
 * Session's dot in their tone.
 */
const KIND_MARK: Partial<Record<TimelineKind, { icon: typeof Clock; said: string }>> = {
  "session.read": { icon: ScrollText, said: "Read" },
  "session.edited": { icon: FilePenLine, said: "Edited" },
  "session.ran": { icon: Terminal, said: "Ran" },
  "job.read": { icon: ScrollText, said: "Read" },
  "job.ran": { icon: Terminal, said: "Ran" },
  "session.subagent": { icon: SplitLine, said: "Subagent" },
};

/** The event's mark on the spine. A subagent that has not finished turns, as the lane's working mark does. */
function Mark({ event, running }: { event: FleetEvent; running: boolean }) {
  const kind = KIND_MARK[event.kind];
  if (kind === undefined) return <i className="armada-tl__mark" data-shape={shapeOf(event)} />;
  const Icon = running ? LoaderCircle : kind.icon;
  return (
    <Tooltip label={running ? "Subagent working" : kind.said}>
      <span className="armada-tl__kind" data-quiet={QUIET_KINDS.has(event.kind) || undefined} data-running={running || undefined}>
        <Icon size={12} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** What a coalesced row opens to, named for the act: files for a read or an edit, commands for a run. */
const itemsOf = (event: FleetEvent): string => (event.kind === "session.ran" || event.kind === "job.ran" ? "commands" : "files");

/** An event's whole line as one string, for a tooltip over a column that cuts it short. */
const wholeLine = (event: FleetEvent): string => [event.says, event.detail].filter((one) => one !== undefined && one !== "").join(" ");

type Picking = {
  /** A press on a row, where the row is the event and not only its subject. */
  onPick?: ((event: FleetEvent) => void) | undefined;
  /** The row held lit. */
  lit?: string | undefined;
};

function Row({ event, now, open, current, running, opened, onToggle, tips, onPick, lit }: { event: FleetEvent; now: number; open: Open; current: Standing | undefined; running: boolean; opened: boolean; onToggle: () => void; tips: boolean } & Picking) {
  const says = (
    <>
      {event.says}
      {event.detail === undefined ? null : <span className="armada-tl__detail">{event.detail}</span>}
    </>
  );
  const items = event.items ?? [];
  return (
    <>
      <li
        className="armada-tl__event"
        data-weight={QUIET_KINDS.has(event.kind) ? "quiet" : undefined}
        data-current={current}
        data-event={event.id}
        data-lit={lit === event.id || undefined}
        style={toneOf(event.tone)}
        {...(onPick === undefined ? {} : { onClick: () => onPick(event) })}
      >
        <Age event={event} now={now} />
        <span className="armada-tl__spine">
          <Mark event={event} running={running} />
        </span>
        <Who subject={event.subject} open={open} />
        {items.length === 0 ? (
          tips ? (
            <Tooltip label={wholeLine(event)} asChild>
              <span className="armada-tl__says">{says}</span>
            </Tooltip>
          ) : (
            <span className="armada-tl__says">{says}</span>
          )
        ) : (
          <Tooltip label={`${opened ? "Hide" : "Show"} ${itemsOf(event)}`} asChild>
            <button type="button" className="armada-tl__opens" aria-expanded={opened} onClick={onToggle}>
              {opened ? <ChevronDown size={12} aria-hidden="true" /> : <ChevronRight size={12} aria-hidden="true" />}
              <span className="armada-tl__says">{says}</span>
            </button>
          </Tooltip>
        )}
      </li>
      {opened
        ? items.map((one) => (
            <li key={one} className="armada-tl__item">
              <span />
              <span className="armada-tl__spine" />
              <span />
              <span className="armada-tl__detail">{one}</span>
            </li>
          ))
        : null}
    </>
  );
}

/** The deck's row: the name and its age over the event, the kind as a glyph, and a mark only where the event is owed or failing. It opens no list of files or commands. */
function StackedRow({ event, now, open, current, onPick, lit }: { event: FleetEvent; now: number; open: Open; current: Standing | undefined } & Picking) {
  return (
    <li className="armada-tl__event" data-stacked data-event={event.id} data-lit={lit === event.id || undefined} style={toneOf(event.tone)} {...(onPick === undefined ? {} : { onClick: () => onPick(event) })}>
      <Glyph subject={event.subject} />
      <Who subject={event.subject} open={open} glyph={false} />
      <Age event={event} now={now} />
      <Tooltip label={wholeLine(event)} asChild>
        <span className="armada-tl__says">
          <span className="armada-tl__what">{event.says}</span>
          {event.detail === undefined ? null : <span className="armada-tl__detail">{event.detail}</span>}
          <Flag event={event} standing={current} />
        </span>
      </Tooltip>
    </li>
  );
}

/** A subagent that started and has not finished, by the id of its newest event: the pair is the Session and the work it was given. */
function unfinished(events: readonly FleetEvent[]): Set<string> {
  const newest = new Map<string, FleetEvent>();
  for (const event of events) {
    if (event.kind !== "session.subagent") continue;
    const key = `${keyOf(event.subject)}|${event.detail ?? ""}`;
    const held = newest.get(key);
    if (held === undefined || Date.parse(event.at) > Date.parse(held.at)) newest.set(key, event);
  }
  return new Set([...newest.values()].filter((one) => one.tone === "run").map((one) => one.id));
}

/** The most events the feed draws, the newest. A busy two hours holds over a thousand, and a row for each froze the deck. */
export const MOST_FEED = 100;

/** The newest `MOST_FEED` events of `lanes`, in blocks by how long ago; the newest of each subject carries how it stands. */
function Feed({ lanes, now, open, tips = false, stacked = false, onPick, lit }: { lanes: readonly Lane[]; now: number; open: Open; /** Each event's whole line, in a tooltip: for a column that cuts it short. */ tips?: boolean; /** Two lines to a row. */ stacked?: boolean } & Picking) {
  const [opened, setOpened] = useState<ReadonlySet<string>>(new Set());
  const toggle = (id: string) => setOpened((held) => (held.has(id) ? new Set([...held].filter((one) => one !== id)) : new Set([...held, id])));
  const standing = new Map(lanes.map((lane) => [lane.events[0]!.id, lane.standing]));
  const running = unfinished(lanes.flatMap((lane) => lane.events));
  const events = lanes
    .flatMap((lane) => lane.events)
    .sort((a, b) => Date.parse(b.at) - Date.parse(a.at))
    .slice(0, MOST_FEED);
  const blocks: { said: string; rows: FleetEvent[] }[] = [];
  for (const event of events) {
    const said = bucketOf(event, now);
    const last = blocks[blocks.length - 1];
    if (last?.said === said) last.rows.push(event);
    else blocks.push({ said, rows: [event] });
  }
  return (
    <>
      {blocks.map((block) => (
        <ol key={block.said} className="armada-tl__block" aria-label={block.said}>
          <li className="armada-tl__part">{block.said}</li>
          {block.rows.map((event) =>
            stacked ? (
              <StackedRow key={event.id} event={event} now={now} open={open} current={standing.get(event.id)} onPick={onPick} lit={lit} />
            ) : (
              <Row key={event.id} event={event} now={now} open={open} current={standing.get(event.id)} running={running.has(event.id)} opened={opened.has(event.id)} onToggle={() => toggle(event.id)} tips={tips} onPick={onPick} lit={lit} />
            ),
          )}
        </ol>
      ))}
    </>
  );
}

const AXIS = [120, 100, 80, 60, 40, 20, 0];

/** A lane's events along the window, the line between them solid while the subject was working and dashed while it was not. */
function Track({ lane, now, press }: { lane: Lane; now: number; press: (event: FleetEvent) => void }) {
  const at = (ms: number) => Math.min(100, Math.max(0, 100 - ((now - ms) / WINDOW) * 100));
  const oldest = [...lane.events].reverse();
  const going = lane.standing !== "done" && lane.standing !== "bad";
  const pieces = oldest.map((event, index) => {
    const next = oldest[index + 1];
    const from = at(Date.parse(event.at));
    const to = next === undefined ? (going ? at(now) : from) : at(Date.parse(next.at));
    const stuck = next === undefined && lane.standing === "stuck";
    return to > from ? { key: event.id, from, to, stuck, solid: next === undefined ? lane.standing === "run" : event.live === true, tone: event.tone } : undefined;
  });
  return (
    <div className="armada-tl__track">
      {pieces.map((piece) =>
        piece === undefined ? null : <i key={piece.key} className="armada-tl__run" data-solid={piece.solid || undefined} data-stuck={piece.stuck || undefined} style={{ ...toneOf(piece.tone), left: `${piece.from}%`, width: `${piece.to - piece.from}%` }} />,
      )}
      {oldest
        .filter((event) => now - Date.parse(event.at) <= WINDOW)
        .map((event) => {
          const said = `${event.says}${event.detail === undefined ? "" : `: ${event.detail}`}, ${ageOf(event.at, now)} ago`;
          const tip = (
            <span className="armada-tl__tip">
              <span>{event.says}</span>
              {event.detail === undefined ? null : <span className="armada-tl__detail">{event.detail}</span>}
              <span className="armada-tl__detail">{clock(event.at)}</span>
            </span>
          );
          return (
            <Tooltip key={event.id} label={tip} card asChild>
              <button type="button" className="armada-tl__pip" data-shape={shapeOf(event)} data-weight={QUIET_KINDS.has(event.kind) ? "quiet" : undefined} aria-label={said} style={{ ...toneOf(event.tone), left: `${at(Date.parse(event.at))}%` }} onClick={() => press(event)} />
            </Tooltip>
          );
        })}
    </div>
  );
}

/** Lanes grouped by how they stand, the time axis along the top. A lane with nothing in the window and nothing under way is left off. */
function Groups({ lanes, now, children }: { lanes: readonly Lane[]; now: number; children: (lane: Lane) => ReactNode }) {
  return (
    <>
      {STANDINGS.map(({ id, said }) => {
        const own = lanes.filter((lane) => lane.standing === id && (id !== "done" || now - lane.last <= WINDOW));
        return own.length === 0 ? null : (
          <ol key={id} className="armada-tl__block" aria-label={said}>
            <li className="armada-tl__part">{said}</li>
            {own.map((lane) => (
              <li key={lane.key} className="armada-tl__lane" data-standing={lane.standing}>
                {children(lane)}
              </li>
            ))}
          </ol>
        );
      })}
    </>
  );
}

function Lanes({ lanes, now, open, press }: { lanes: readonly Lane[]; now: number; open: Open; press: (event: FleetEvent) => void }) {
  return (
    <>
      <div className="armada-tl__axis" role="presentation">
        <span />
        <span className="armada-tl__scale">
          {AXIS.map((minutes) => (
            <span key={minutes} style={{ left: `${100 - (minutes / MINUTES) * 100}%` }}>
              {minutes === 0 ? "now" : `${minutes}m`}
            </span>
          ))}
        </span>
      </div>
      <Groups lanes={lanes} now={now}>
        {(lane) => (
          <>
            <span className="armada-tl__label">
              <Stand lane={lane} now={now} />
              <Who subject={lane.subject} open={open} />
            </span>
            <Track lane={lane} now={now} press={press} />
          </>
        )}
      </Groups>
    </>
  );
}

function Split({ lanes, now, open }: { lanes: readonly Lane[]; now: number; open: Open }) {
  const [picked, setPicked] = useState<string>();
  const one = lanes.find((lane) => lane.key === picked);
  return (
    <div className="armada-tl__split">
      <nav className="armada-tl__subjects" aria-label="Subjects">
        <Groups lanes={lanes} now={now}>
          {(lane) => (
            <button type="button" className="armada-tl__subject" aria-pressed={lane.key === picked} aria-label={`${KIND[lane.subject.kind]}: ${titleOf(lane.subject)}`} onClick={() => setPicked(lane.key === picked ? undefined : lane.key)}>
              <Stand lane={lane} now={now} />
              <Glyph subject={lane.subject} />
              <span className="armada-tl__title">{titleOf(lane.subject)}</span>
              <span className="armada-tl__age">{ageOf(lane.events[0]!.at, now)}</span>
            </button>
          )}
        </Groups>
      </nav>
      <div className="armada-tl__events">
        {one === undefined ? null : (
          <div className="armada-tl__picked">
            <Who subject={one.subject} open={open} />
            {one.subject.kind === "main" ? null : (
              <Button variant="ghost" size="sm" onClick={() => open(one.subject)}>
                Open
              </Button>
            )}
          </div>
        )}
        <Feed lanes={one === undefined ? lanes : [one]} now={now} open={open} />
      </div>
    </div>
  );
}

/**
 * The lanes the Timeline and the feed draw: Fleet's events named from what the window holds, narrowed by the filter and by `keep`.
 * `most` keeps the newest that many events **of the subjects `keep` and `of` leave**, before the lanes are read, so a narrowed feed is
 * never emptied by newer events of subjects it does not show.
 */
function useLanes(now: number, jobs: readonly JobSummary[], views: readonly MergeLineView[], filter: DashboardTab, keep: ((subject: Subject) => boolean) | undefined, of?: Of, most?: number): Lane[] {
  const wire = useFleetEvents();
  const sessions = useSessions();
  const events = useMemo(() => {
    // Only a pull request's opening and its merge carry its branch, so the other entries read it from those.
    const branches = new Map(wire.flatMap((one) => (one.subject.kind === "pull_request" && one.detail !== undefined && (one.kind === "pr.opened" || one.kind === "pr.merged") ? [[one.subject.number, one.detail] as const] : [])));
    return wire.flatMap((one) => {
      const drawn = fromWire(one, {
        job: (id) => {
          const job = jobs.find((row) => row.id === id);
          return job === undefined ? undefined : plainTitle(job);
        },
        session: (id) => sessions.find((row) => row.id === id)?.title,
        pull: (number) => timelineOwner(views, jobs, sessions, number, branches.get(number)),
      });
      return drawn === undefined ? [] : [drawn];
    });
  }, [wire, jobs, views, sessions]);
  return useMemo(() => {
    const drawn =
      most === undefined
        ? events
        : events
            .filter((one) => (of !== undefined ? one.subject.kind === of.kind && "id" in one.subject && one.subject.id === of.id : keep === undefined || keep(one.subject)))
            .sort((a, b) => Date.parse(b.at) - Date.parse(a.at))
            .slice(0, most);
    const all = lanesOf(drawn, now, (id) => {
      const held = sessions.find((one) => one.id === id);
      return held === undefined ? undefined : stateOf(held).state;
    });
    // One Job or Session picked: its lane alone, whatever its standing. A Session's subagents are events on its own lane.
    if (of !== undefined) return all.filter((lane) => lane.subject.kind === of.kind && lane.subject.id === of.id);
    return lanesFor(all, filter).filter((lane) => keep === undefined || keep(lane.subject));
  }, [events, now, filter, sessions, keep, of?.kind, of?.id, most]);
}

const NO_VIEWS: readonly MergeLineView[] = [];

/** The Job or Session the feed narrows to. */
export type Of = { kind: "job" | "session"; id: string };

export function Timeline({
  now,
  jobs,
  views = NO_VIEWS,
  filter,
  recessed,
  keep,
  onOpenJob,
  onOpenSession,
  onOpenLink,
}: {
  now: number;
  /** The Board's Jobs, for the name of a Job Fleet gave an entry no title for. */
  jobs: readonly JobSummary[];
  /** The merge lines, which say who a pull request belongs to. Without them a pull request's events have no owner. */
  views?: readonly MergeLineView[];
  /** The Cockpit's filter: what needs him, what is not over, what is. */
  filter: DashboardTab;
  /** A call is in front, and the glass is dimmed behind it. */
  recessed: boolean;
  /** Narrows the lanes to the subjects it keeps: the flight deck's own filters. Absent keeps every one. */
  keep?: (subject: Subject) => boolean;
  onOpenJob: (id: string) => void;
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
}) {
  const [take, setTake] = useTake();
  const lanes = useLanes(now, jobs, views, filter, keep);
  // Lanes open at now, the right-hand end; the older hour is a scroll back.
  const body = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (take === "lanes" && body.current !== null) body.current.scrollLeft = body.current.scrollWidth;
  }, [take]);
  const open: Open = (subject) => {
    if (subject.kind === "job") onOpenJob(subject.id);
    else if (subject.kind === "session") onOpenSession(subject.id);
  };
  /** A pull request's mark opens the pull request; any other opens its subject. */
  const press = (event: FleetEvent) => (event.pull?.url === undefined ? open(event.subject) : onOpenLink(event.pull.url));
  return (
    <div className="armada-view__timeline" data-recessed={recessed ? "" : undefined}>
      <section className="armada-tl" aria-label="Timeline" data-take={take}>
        <header className="armada-tl__head">
          <Tabs items={TAKES} value={take} onChange={(id) => setTake(id as Take)} />
          <span className="armada-tl__legend">
            {LEGEND.map(([tone, said]) => (
              <span key={tone}>
                <i style={toneOf(tone)} />
                {said}
              </span>
            ))}
          </span>
        </header>
        <div className="armada-tl__body" key={take} ref={body} style={{ ["--span" as string]: MINUTES / SHOWN } as CSSProperties}>
          {take === "feed" ? <Feed lanes={lanes} now={now} open={open} /> : take === "lanes" ? <Lanes lanes={lanes} now={now} open={open} press={press} /> : <Split lanes={lanes} now={now} open={open} />}
        </div>
      </section>
    </div>
  );
}

/** The Feed take on its own, for a narrow column: one row for each event, newest first. A press names the Job or Session it is about. */
export function ActivityFeed({
  now,
  jobs,
  views = NO_VIEWS,
  filter,
  keep,
  onOpenJob,
  onOpenSession,
  onPick,
  lit,
  stacked,
  of,
}: {
  now: number;
  jobs: readonly JobSummary[];
  views?: readonly MergeLineView[];
  filter: DashboardTab;
  keep?: (subject: Subject) => boolean;
  /** Narrows the feed to this Job or Session alone: the one open on the flight deck's stage. Absent keeps every lane `keep` does. */
  of?: Of;
  onOpenJob: (id: string) => void;
  onOpenSession: (id: string) => void;
  /** The flight deck's: a press on a row picks the event, and the name opens nothing. */
  onPick?: (event: FleetEvent) => void;
  /** The row held lit, by event id. */
  lit?: string;
  /** The deck's two-line rows. */
  stacked?: boolean;
}) {
  const lanes = useLanes(now, jobs, views, filter, keep, of, MOST_FEED);
  const open: Open = (subject) => {
    if (onPick !== undefined) return;
    if (subject.kind === "job") onOpenJob(subject.id);
    else if (subject.kind === "session") onOpenSession(subject.id);
  };
  return (
    <section className="armada-tl" aria-label="Activity feed" data-take="feed">
      <div className="armada-tl__body">
        <Feed lanes={lanes} now={now} open={open} tips stacked={stacked === true} onPick={onPick} lit={lit} />
      </div>
    </section>
  );
}

/** How wide a column of the graph is, how many there are, and so how far back it reaches: twenty of three minutes, the last hour. */
const SPAN = 3 * 60_000;
const COLUMNS = 20;
/** The newest events the graph counts. A cell needs no row of its own, so this reaches further than the feed's. */
const MOST_GRAPH = 500;
/** The newest events a cell's card lists. A card cannot scroll, so a busier cell is cut here. */
const MOST_CARD = 8;
/** How much a coalesced read, edit or command weighs beside a turn, a step or an ask. */
const QUIET_WEIGHT = 0.4;
/** The steps of the blue ramp, an empty cell being none of them. */
const LEVELS = 5;

/** `events` in the last hour, in the span each falls in: the oldest span first, `now` at the right edge of the last. Each is newest first. */
function spansOf(events: readonly FleetEvent[], now: number): FleetEvent[][] {
  const spans = Array.from({ length: COLUMNS }, () => [] as FleetEvent[]);
  for (const event of events) {
    const back = Math.floor((now - Date.parse(event.at)) / SPAN);
    if (back < COLUMNS) spans[COLUMNS - 1 - Math.max(0, back)]!.push(event);
  }
  for (const span of spans) span.sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
  return spans;
}

/** What a cell weighs: its events, the coalesced ones less. */
const weighOf = (events: readonly FleetEvent[]): number => events.reduce((sum, event) => sum + (QUIET_KINDS.has(event.kind) ? QUIET_WEIGHT : 1), 0);

/** The tone a cell takes over the blue ramp, where it holds what is owed him or failing. Failing first. */
const owedOf = (events: readonly FleetEvent[]): EventTone | undefined => (events.some((event) => event.tone === "bad") ? "bad" : events.some((event) => event.tone === "ask") ? "ask" : undefined);

/** What a cell's card says: its newest events, each as the stacked row draws one, the glyph in the event's colour. Nothing in it can be pressed. */
function Hour({ events, now, lit }: { events: readonly FleetEvent[]; now: number; lit: string | undefined }) {
  return (
    <span className="armada-tl__tip armada-tl__hour">
      {events.slice(0, MOST_CARD).map((event) => {
        const Kind = GLYPH[event.subject.kind];
        const Owed = event.tone === "ask" ? STANDING_MARK.ask.icon : event.tone === "bad" ? STANDING_MARK.bad.icon : undefined;
        return (
          <span key={event.id} className="armada-tl__moment" data-lit={lit === event.id || undefined} data-weight={QUIET_KINDS.has(event.kind) ? "quiet" : undefined} style={toneOf(event.tone)}>
            <Kind size={14} aria-hidden="true" className="armada-tl__kindmark" />
            <span className="armada-tl__title">{titleOf(event.subject)}</span>
            <time className="armada-tl__age" dateTime={event.at}>
              {ageOf(event.at, now)}
            </time>
            <span className="armada-tl__says">
              <span className="armada-tl__what">{event.says}</span>
              {event.detail === undefined ? null : <span className="armada-tl__detail">{event.detail}</span>}
              {Owed === undefined ? null : <Owed size={14} aria-hidden="true" className="armada-tl__flag" />}
            </span>
          </span>
        );
      })}
    </span>
  );
}

/**
 * The deck's Activity as a heatmap and no text: a row for each Job or Session, the most recently active on top, and a cell for each
 * three minutes of the last hour, the newest at the right. A cell is bluer the more happened in it, the quiet kinds counting less, and
 * takes its tone where it holds an ask or a failure. A faint tick under the right edge is now. **Hovering or focusing a cell shows
 * that subject's events in it, and pressing it picks the newest**: the card is a tooltip and holds nothing to press. Rows are cut at
 * the panel's height. Takes what the feed takes, so the Job or Session open narrows it to one row.
 */
export function ActivityGraph({
  now,
  jobs,
  views = NO_VIEWS,
  filter,
  keep,
  onPick,
  lit,
  of,
}: {
  now: number;
  jobs: readonly JobSummary[];
  views?: readonly MergeLineView[];
  filter: DashboardTab;
  keep?: (subject: Subject) => boolean;
  /** Narrows the graph to this Job or Session alone: the one open on the flight deck's stage. */
  of?: Of;
  /** A press on a cell picks the newest event in it. */
  onPick: (event: FleetEvent) => void;
  /** The event held lit, by id: its cell is ringed. */
  lit?: string;
}) {
  const lanes = useLanes(now, jobs, views, filter, keep, of, MOST_GRAPH);
  const rows = useMemo(
    () =>
      lanes
        .filter((lane) => lane.subject.kind !== "main")
        .map((lane) => ({ lane, spans: spansOf(lane.events, now) }))
        .filter((row) => row.spans.some((span) => span.length > 0))
        .sort((a, b) => b.lane.last - a.lane.last),
    [lanes, now],
  );
  const busiest = Math.max(1, ...rows.flatMap((row) => row.spans.map(weighOf)));
  // The rows that fit: no row is shorter than a cell is wide.
  const plot = useRef<HTMLDivElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  const [fit, setFit] = useState<number>();
  useLayoutEffect(() => {
    const held = plot.current;
    if (held === null || typeof ResizeObserver === "undefined") return;
    const measure = () => {
      const gap = Number.parseFloat(getComputedStyle(grid.current ?? held).columnGap) || 0;
      const cell = (held.clientWidth - gap * (COLUMNS - 1)) / COLUMNS;
      if (cell > 0) setFit(Math.max(1, Math.floor((held.clientHeight + gap) / (cell + gap))));
    };
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(held);
    return () => watch.disconnect();
  }, []);
  const shown = rows.slice(0, fit);
  return (
    <div className="armada-tl armada-tl__graph" role="group" aria-label="Last hour" data-take="graph">
      <div className="armada-tl__plot" ref={plot}>
        <div className="armada-tl__cells" ref={grid} style={{ ["--rows" as string]: shown.length } as CSSProperties}>
          {shown.map(({ lane, spans }) => (
            <div key={lane.key} className="armada-tl__row" role="group" aria-label={titleOf(lane.subject)}>
              {spans.map((events, index) => {
                const newest = events[0];
                if (newest === undefined) return <i key={index} className="armada-tl__cell" data-level="0" aria-hidden="true" />;
                const owed = owedOf(events);
                const level = Math.min(LEVELS, Math.max(1, Math.ceil((weighOf(events) / busiest) * LEVELS)));
                return (
                  <Tooltip key={index} label={<Hour events={events} now={now} lit={lit} />} card beside asChild>
                    <button
                      type="button"
                      className="armada-tl__cell"
                      data-level={level}
                      data-owed={owed}
                      data-lit={events.some((event) => event.id === lit) || undefined}
                      style={owed === undefined ? undefined : toneOf(owed)}
                      aria-label={`${titleOf(newest.subject)}: ${newest.says}`}
                      onClick={() => onPick(newest)}
                    />
                  </Tooltip>
                );
              })}
            </div>
          ))}
          <i className="armada-tl__now" aria-hidden="true" />
        </div>
      </div>
    </div>
  );
}

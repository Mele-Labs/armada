// The Cockpit's other face: what Fleet did, newest first, one row per event, each naming what it happened
// to. Three takes of it share one set of marks, Lanes first and his last pick kept: a feed of every
// event, a lane for each Session with its events along the last two hours,
// and the lanes beside the feed. A pull request is no lane: its events are marks on the track of the Session
// that owns it, and one nobody owns is left out. The filter above narrows it as it narrows the
// grid. What it draws is Fleet's timeline (`fleet-events.tsx`). Nothing here moves but a subject that is working.

import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { BellRing, ChevronDown, ChevronRight, CircleCheck, CircleDashed, CircleX, Clock, FilePenLine, GitMerge, LoaderCircle, Power, Reply, ScrollText, Split as SplitLine, SquareTerminal, Terminal } from "lucide-react";
import { Button, Inspect, InspectCard, Tabs, Tooltip } from "@armada/components";
import type { DashboardTab } from "@armada/overview";
import type { TimelineKind } from "@armada/protocol";
import type { MergeLineView } from "@armada/screens";

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
  { id: "lanes", label: "Lanes", hint: "A row for each Session, its events along the last two hours" },
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

const KIND: Record<Subject["kind"], string> = { session: "Session", main: "Main" };
export const GLYPH = { session: SquareTerminal, main: GitMerge };

/** What an event's mark is shaped as: a pull request's keeps its own shape on its owner's track. */
const shapeOf = (event: FleetEvent): string => (event.pull === undefined ? event.subject.kind : "pull_request");

export const toneOf = (tone: EventTone) => ({ ["--tone" as string]: TONE[tone] }) as CSSProperties;
const CLOCK = new Intl.DateTimeFormat([], { hour: "numeric", minute: "numeric", second: "numeric", hour12: false });
const clock = (at: string) => CLOCK.format(new Date(at));

/** Where a subject opens: the Session. Main opens nothing. */
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
function Who({ subject, open, children }: { subject: Subject; open: Open; children?: ReactNode }) {
  const inside = (
    <>
      <Glyph subject={subject} />
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

export const STANDING_MARK: Record<Standing, { icon: typeof Clock; said: string }> = {
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
      <span className="armada-tl__standing" role="img" aria-label={label} data-standing={lane.standing} data-reply={lane.session === "replied" || undefined} style={toneOf(tone)}>
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

const bucketOf = (event: FleetEvent, now: number): string => {
  const minutes = (now - Date.parse(event.at)) / 60_000;
  return minutes < 3 ? "Just now" : minutes < 15 ? "Last 15 minutes" : minutes < 60 ? "Last hour" : "Earlier";
};

/**
 * The kinds that wear a glyph in the spine where the rest wear a dot: the coalesced work of a Session, and
 * its subagents. A waiting, a resolution and a Session started by another keep the
 * Session's dot in their tone.
 */
const KIND_MARK: Partial<Record<TimelineKind, { icon: typeof Clock; said: string }>> = {
  "session.read": { icon: ScrollText, said: "Read" },
  "session.edited": { icon: FilePenLine, said: "Edited" },
  "session.ran": { icon: Terminal, said: "Ran" },
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
const itemsOf = (event: FleetEvent): string => (event.kind === "session.ran" ? "commands" : "files");

function Row({ event, now, open, current, running, opened, onToggle }: { event: FleetEvent; now: number; open: Open; current: Standing | undefined; running: boolean; opened: boolean; onToggle: () => void }) {
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
        style={toneOf(event.tone)}
      >
        <Age event={event} now={now} />
        <span className="armada-tl__spine">
          <Mark event={event} running={running} />
        </span>
        <Who subject={event.subject} open={open} />
        {items.length === 0 ? (
          <span className="armada-tl__says">{says}</span>
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
const MOST_FEED = 100;

/** The newest `MOST_FEED` events of `lanes`, in blocks by how long ago; the newest of each subject carries how it stands. */
function Feed({ lanes, now, open }: { lanes: readonly Lane[]; now: number; open: Open }) {
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
          {block.rows.map((event) => (
            <Row key={event.id} event={event} now={now} open={open} current={standing.get(event.id)} running={running.has(event.id)} opened={opened.has(event.id)} onToggle={() => toggle(event.id)} />
          ))}
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
          const mark = KIND_MARK[event.kind];
          const tip = (
            <InspectCard
              kind={{ Glyph: GLYPH[event.subject.kind], label: KIND[event.subject.kind] }}
              where={clock(event.at)}
              title={titleOf(event.subject)}
              state={{ Glyph: mark?.icon ?? Clock, said: event.says, tone: TONE[event.tone] }}
              facts={[...(event.detail === undefined ? [] : [{ label: "Detail", value: event.detail, mono: true }]), { label: "Ago", value: ageOf(event.at, now), mono: true }]}
              keys={[{ key: "Enter", verb: "Open" }]}
            />
          );
          return (
            <Inspect key={event.id} card={tip} name={said} pin={false}>
              <button type="button" className="armada-tl__pip" data-shape={shapeOf(event)} data-weight={QUIET_KINDS.has(event.kind) ? "quiet" : undefined} aria-label={said} style={{ ...toneOf(event.tone), left: `${at(Date.parse(event.at))}%` }} onClick={() => press(event)} />
            </Inspect>
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
export function useLanes(now: number, views: readonly MergeLineView[], filter: DashboardTab, keep: ((subject: Subject) => boolean) | undefined, of?: Of, most?: number): Lane[] {
  const wire = useFleetEvents();
  const sessions = useSessions();
  const events = useMemo(() => {
    // Only a pull request's opening and its merge carry its branch, so the other entries read it from those.
    const branches = new Map(wire.flatMap((one) => (one.subject.kind === "pull_request" && one.detail !== undefined && (one.kind === "pr.opened" || one.kind === "pr.merged") ? [[one.subject.number, one.detail] as const] : [])));
    return wire.flatMap((one) => {
      const drawn = fromWire(one, {
        session: (id) => sessions.find((row) => row.id === id)?.title,
        pull: (number) => timelineOwner(views, sessions, number, branches.get(number)),
      });
      return drawn === undefined ? [] : [drawn];
    });
  }, [wire, views, sessions]);
  return useMemo(() => {
    const drawn =
      most === undefined
        ? events
        : events
            .filter((one) => (of !== undefined ? one.subject.kind === of.kind && "id" in one.subject && one.subject.id === of.id : keep === undefined || keep(one.subject)))
            .sort((a, b) => Date.parse(b.at) - Date.parse(a.at))
            .slice(0, most);
    const all = lanesOf(drawn, (id) => {
      const held = sessions.find((one) => one.id === id);
      return held === undefined ? undefined : stateOf(held).state;
    });
    // One Session picked: its lane alone, whatever its standing. A Session's subagents are events on its own lane.
    if (of !== undefined) return all.filter((lane) => lane.subject.kind === of.kind && lane.subject.id === of.id);
    return lanesFor(all, filter).filter((lane) => keep === undefined || keep(lane.subject));
  }, [events, now, filter, sessions, keep, of?.kind, of?.id, most]);
}

const NO_VIEWS: readonly MergeLineView[] = [];

/** The Session the feed narrows to. */
export type Of = { kind: "session"; id: string };

export function Timeline({
  now,
  views = NO_VIEWS,
  filter,
  recessed,
  keep,
  onOpenSession,
  onOpenLink,
}: {
  now: number;
  /** The merge lines, which say who a pull request belongs to. Without them a pull request's events have no owner. */
  views?: readonly MergeLineView[];
  /** The Cockpit's filter: what needs him, what is not over, what is. */
  filter: DashboardTab;
  /** A call is in front, and the glass is dimmed behind it. */
  recessed: boolean;
  /** Narrows the lanes to the subjects it keeps: the flight deck's own filters. Absent keeps every one. */
  keep?: (subject: Subject) => boolean;
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
}) {
  const [take, setTake] = useTake();
  const lanes = useLanes(now, views, filter, keep);
  // Lanes open at now, the right-hand end; the older hour is a scroll back.
  const body = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (take === "lanes" && body.current !== null) body.current.scrollLeft = body.current.scrollWidth;
  }, [take]);
  const open: Open = (subject) => {
    if (subject.kind === "session") onOpenSession(subject.id);
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

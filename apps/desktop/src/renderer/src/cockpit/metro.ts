// The Bays filter as metro lines: each piece of work in a bay, and each waiting for one, a line through
// the same stations, Waiting to main. The line takes the work's own status colour; each stop it has
// reached takes the state at that station; the stop it is at now says what to know inside it, its place
// in the merge queue, ? when it asks, ! when its checks fail. Every stop opens a card. Pure, so where a
// piece of work stands and what its stops say are checked without drawing a line.

import type { Item } from "../Dashboard";
import type { Bay, BaysRead } from "./bays";
import type { Dot } from "./horizon";

export const STATIONS = ["Waiting", "In a bay", "Building", "Pull request", "Merge queue", "main"] as const;
export const MAIN = STATIONS.length - 1;

/** The status colours, Bridge's own: working, needs you, failing, passed, waiting, blocked. */
export type Tone = "run" | "ask" | "bad" | "ok" | "idle" | "esc";

/** What a card can do: open the Job, the Session, or the pull request on the forge. */
export type Act = { kind: "job"; id: string; label: string } | { kind: "session"; id: string; label: string } | { kind: "link"; url: string; label: string };

export type Card = { tone: Tone; pill: string; title: string; rows: readonly (readonly [string, string, boolean?])[]; acts: readonly Act[] };

export type Stop = { tone: Tone; card: Card; glyph?: string; progress?: number };

export type Line = {
  key: string;
  /** The tile it is, so a press selects it as the grid and the map do. */
  item: Item | undefined;
  bay: number | undefined;
  title: string;
  session: boolean;
  tone: Tone;
  /** The station it is at now. */
  reach: number;
  /** One per station; absent where the work has no stop there. */
  stops: readonly (Stop | undefined)[];
  /** The first station it has a stop at: a pull request opened outside a bay starts at Pull request. */
  from?: number;
  /** What a press on its title opens: its Job, or its Session, or a pull request on the forge. None for a terminal Armada does not know. */
  open: Act | undefined;
  /** A row tinted for what it wants: the owner's answer, or its failing checks. */
  wants: "you" | "fixing" | undefined;
  /** Autopilot's: the line wears its glyph. Set by `autopilot-metro.ts`, only while one is on. */
  autopilot?: boolean;
  /** Its stop at Next up, where Autopilot picked it and its Job has not started. */
  nextUp?: Stop;
};

export type Metro = {
  /** One block of lines per repository with bays, furthest along first. */
  /** `outside` is the pull requests on the repository's merge line that no bay holds: opened by hand, or from another machine. */
  harbours: readonly { manifest: string; name: string; lines: readonly Line[]; outside: readonly Line[] }[];
  waiting: readonly Line[];
  /** Autopilot's picks not started, at Next up before Waiting: only while an Autopilot is on. */
  nextUp?: readonly Line[];
  /** How many lines are at each station, by tone, for the station's header. */
  counts: readonly (readonly (readonly [Tone, number])[])[];
  bays: { held: number; free: number; closed: number; strip: readonly ("held" | "free" | "closed")[]; names: readonly string[] };
  main: "green" | "red" | undefined;
};

const HUE: Record<Item["hue"], Tone> = { running: "run", ask: "ask", issue: "ask", queued: "idle", ok: "ok", bad: "bad" };
const PULL: Record<Dot["state"], Tone> = { running: "run", failing: "bad", ready: "ok", blocked: "esc", queued: "idle", open: "idle" };

const two = (n: number) => String(n).padStart(2, "0");
const ordinal = (place: number) => `${place}${place === 1 ? "st" : place === 2 ? "nd" : place === 3 ? "rd" : "th"}`;

/** Where a line stands in the merge queue, for ordering: one with no place sorts after those with one. */
const placeOf = (line: Line) => Number(line.stops[4]?.glyph ?? Infinity);

/** How long ago `since` was, in the Board's words: 41m, 2h 05m. */
export function heldFor(since: string | undefined, now: number): string | undefined {
  const at = since === undefined ? NaN : Date.parse(since);
  if (Number.isNaN(at)) return undefined;
  const minutes = Math.max(0, Math.round((now - at) / 60_000));
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, "0")}m`;
}

const sessionOf = (item: Item | undefined) => (item?.key.startsWith("session:") === true ? item.key.split(":")[1] : undefined);

/** The acts on the work itself: its Job, or its Session. */
function opens(item: Item | undefined): Act[] {
  if (item?.job !== undefined) return [{ kind: "job", id: item.job.id, label: "Open Job" }];
  const session = sessionOf(item);
  return session === undefined ? [] : [{ kind: "session", id: session, label: "Open Session" }];
}

/** What the work in a bay opens: the Job by its id, the Session by its own, whether or not it has a tile. */
function opensIn(holder: Bay["holder"]): Act[] {
  if (holder.kind === "job") return [{ kind: "job", id: holder.id, label: "Open Job" }];
  if (holder.kind === "session" && holder.session !== undefined) return [{ kind: "session", id: holder.session.id, label: "Open Session" }];
  return [];
}

/** A pull request's link to the forge, where its dot has one. */
const linkOf = (dot: Dot): Act[] => (dot.act?.kind === "link" ? [{ kind: "link", url: dot.act.url, label: `Open ${dot.card.heading}` }] : []);

/** Its stops at Pull request and, while it holds a place there, Merge queue. */
function pullStops(dot: Dot): [Stop, Stop | undefined] {
  const link = linkOf(dot);
  const owner = dot.card.owner;
  const place = dot.queued ? dot.card.place : undefined;
  return [
    {
      tone: PULL[dot.state],
      ...(dot.state === "failing" ? { glyph: "!" } : {}),
      card: {
        tone: PULL[dot.state],
        pill: dot.card.says,
        title: dot.card.heading.startsWith("#") ? `Pull request ${dot.card.heading}` : "Landing",
        rows: [...(dot.card.title === undefined ? [] : [["Title", dot.card.title] as const]), ["Branch", dot.card.branch, true], ...(dot.card.check === undefined ? [] : [["Checks", dot.card.check] as const]), ...(owner === undefined ? [] : [[owner.kind === "job" ? "Job" : "Session", owner.title] as const])],
        acts: [...link, ...(owner === undefined ? [] : [{ kind: owner.kind, id: owner.id, label: owner.kind === "job" ? "Open Job" : "Open Session" } as Act])],
      },
    },
    place === undefined
      ? undefined
      : {
          tone: dot.state === "running" ? "run" : "idle",
          glyph: String(place),
          card: { tone: dot.state === "running" ? "run" : "idle", pill: dot.card.says, title: `${ordinal(place)} in the merge queue`, rows: [["Pull request", dot.card.heading]], acts: link },
        },
  ];
}

/** A pull request no bay holds: it joins the lines at Pull request, and its title opens it on the forge. */
function lineOfPull(dot: Dot): Line {
  const [pull, queue] = pullStops(dot);
  const tone = PULL[dot.state];
  return {
    key: dot.key,
    item: undefined,
    bay: undefined,
    // Its branch, as the Merge line page names it; the forge's title is on its card.
    title: dot.card.branch,
    session: false,
    tone,
    reach: queue === undefined ? 3 : 4,
    from: 3,
    stops: [undefined, undefined, undefined, pull, queue, undefined],
    open: linkOf(dot)[0],
    wants: dot.state === "failing" ? "fixing" : undefined,
  };
}

function lineOfBay(bay: Bay, now: number): Line | undefined {
  const { holder } = bay;
  if (holder.kind !== "job" && holder.kind !== "session") return undefined;
  const item = holder.item;
  const own = opensIn(holder);
  const tone = item === undefined ? "idle" : HUE[item.hue];
  const dot = bay.dot;
  const queued = dot !== undefined && dot.queued && dot.card.place !== undefined;
  const reach = queued ? 4 : dot !== undefined ? 3 : 2;
  const held = heldFor(bay.since, now);
  const asks = !queued && dot === undefined && tone === "ask";
  const steps = item?.steps;
  const at = item?.stepAt;
  const stepped = steps !== undefined && at !== undefined && steps.length > 0;
  const riders = bay.riders.map((one) => one.session.title ?? one.session.id);
  const stops: (Stop | undefined)[] = [
    { tone: "ok", card: { tone: "ok", pill: "in", title: `Took bay ${two(bay.slot)}`, rows: held === undefined ? [] : [["Since", `${held} ago`]], acts: [] } },
    {
      tone,
      card: {
        tone,
        pill: `bay ${two(bay.slot)}`,
        title: `In bay ${two(bay.slot)}`,
        rows: [
          ...(bay.branch === undefined ? [] : [["Branch", bay.branch, true] as const]),
          ...(bay.behind === undefined ? [] : [["Behind main", bay.behind === 0 ? "level" : `${bay.behind} commits`] as const]),
          ...(held === undefined ? [] : [["Held", held] as const]),
          ...(holder.kind === "session" && holder.by !== undefined ? [["Held by", holder.by] as const] : []),
          ...(riders.length === 0 ? [] : [["Sessions", riders.join(", ")] as const]),
        ],
        acts: [...own, ...bay.riders.flatMap((one): Act[] => [{ kind: "session", id: one.session.id, label: `Open ${one.session.title ?? "Session"}` }]).slice(0, 2)],
      },
    },
    {
      tone: asks ? "ask" : "run",
      ...(asks ? { glyph: "?" } : {}),
      ...(stepped ? { progress: Math.round(((at + 1) / steps.length) * 100) } : {}),
      card: {
        tone: asks ? "ask" : "run",
        pill: asks ? "asks you" : holder.kind === "session" ? "working" : "building",
        title: stepped ? `${steps[at]?.label ?? "Working"}, step ${at + 1} of ${steps.length}` : holder.title,
        rows: [["Now", item?.fact ?? "—"]],
        acts: own,
      },
    },
    ...(dot === undefined ? [undefined, undefined] : pullStops(dot)),
    undefined,
  ];
  return {
    key: bay.key,
    item,
    bay: bay.slot,
    title: holder.title,
    session: holder.kind === "session",
    tone,
    reach,
    stops,
    open: own[0],
    wants: dot?.state === "failing" ? "fixing" : tone === "ask" ? "you" : undefined,
  };
}

function lineOfWaiting(item: Item, bay: Bay | undefined): Line {
  return {
    key: item.key,
    item,
    bay: undefined,
    title: item.title,
    session: item.session === true,
    tone: "idle",
    reach: 0,
    stops: [
      { tone: "idle", card: { tone: "idle", pill: "waiting", title: "Waiting for a bay", rows: [["Goes to", bay === undefined ? "the next bay that frees" : `bay ${two(bay.slot)}, the first free one`]], acts: opens(item) } },
    ],
    open: opens(item)[0],
    wants: undefined,
  };
}

export function metroOf(read: BaysRead, now: number): Metro {
  const harbours = read.harbours.map((harbour) => ({
    manifest: harbour.manifest,
    name: harbour.name,
    lines: harbour.bays
      .flatMap((bay) => {
        const line = lineOfBay(bay, now);
        return line === undefined ? [] : [line];
      })
      .sort((a, b) => b.reach - a.reach || (a.bay ?? 0) - (b.bay ?? 0)),
    outside: harbour.line
      .filter((dot) => dot.key.startsWith("pull:") && !harbour.bays.some((bay) => bay.dot?.key === dot.key))
      .map(lineOfPull)
      .sort((a, b) => b.reach - a.reach || placeOf(a) - placeOf(b)),
  }));
  const free = (manifest: string | undefined) => read.harbours.find((one) => one.manifest === manifest)?.bays.find((bay) => bay.holder.kind === "free");
  const waiting = read.waiting.map((item) => lineOfWaiting(item, free(item.job?.owner_manifest_id)));
  const every = [...harbours.flatMap((one) => [...one.lines, ...one.outside]), ...waiting];
  const counts = STATIONS.map((_, station) => {
    const at = every.filter((line) => line.reach === station);
    const tally = new Map<Tone, number>();
    at.forEach((line) => {
      const tone = line.stops[station]?.tone ?? line.tone;
      tally.set(tone, (tally.get(tone) ?? 0) + 1);
    });
    return [...tally];
  });
  const all = read.harbours.flatMap((one) => one.bays);
  const kind = (bay: Bay) => (bay.holder.kind === "free" ? "free" : bay.holder.kind === "closed" || bay.holder.kind === "other" ? "closed" : "held");
  return {
    harbours,
    waiting,
    counts,
    bays: {
      held: all.filter((bay) => kind(bay) === "held").length,
      free: all.filter((bay) => kind(bay) === "free").length,
      closed: all.filter((bay) => kind(bay) === "closed").length,
      strip: all.map(kind),
      // One colour for every held bay: the strip says held, free or closed, and the lines say the rest.
      names: all.map((bay) => `Bay ${two(bay.slot)}: ${bay.holder.kind === "job" || bay.holder.kind === "session" ? bay.holder.title : bay.holder.kind === "other" ? bay.holder.said : bay.holder.kind}`),
    },
    main: read.harbours[0]?.main,
  };
}

/** Every line's tile in the order they are drawn, so the keys step down the lines as the eye does. */
export const orderOf = (metro: Metro): Item[] => [...metro.harbours.flatMap((one) => one.lines), ...metro.waiting].flatMap((line) => (line.item === undefined ? [] : [line.item]));

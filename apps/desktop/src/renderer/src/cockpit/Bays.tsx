// The Cockpit's Bays: each repository a ship, its worktree bays the rooms of its hull, laid in as many
// columns as its width holds, the work in them marked as the map marks it
// (a Job a star, a Session a diamond, lit by how much it wants the owner) with the Sessions riding with
// a Job in orbit round it. What waits for a bay queues on the left; on the right the merge line stands
// as cards, and each bay with a pull request is wired out to its card through the hull's conduits, the
// gaps between bays, so no wire crosses a bay. A free bay is empty with its lamp lit; a closed one dark.
// What moves is what is still working: a working Session's orbit, a wire whose checks run.

import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";
import { Button, Tooltip } from "@armada/components";

import type { Item } from "../Dashboard";
import type { Bay, BaysRead } from "./bays";
import { burn } from "./FleetMap";
import type { Dot } from "./horizon";

/** A Session in orbit round the Job it works on: its hue, and whether it is mid-turn. */
type Pod = { key: string; hue: string; working: boolean };

/** A Job's star or a Session's diamond, the map's own marks at the size of a bay, its riders in orbit. */
function Mark({ item, session, pods = [] }: { item: Item | undefined; session: boolean; pods?: readonly Pod[] }) {
  const lit = item === undefined ? { r: 4.5, halo: 0, alpha: 0.75 } : burn(item);
  const r = Math.min(lit.r, 7);
  return (
    <svg className="armada-ship__mark armada-star" data-hue={item?.hue ?? "queued"} viewBox="-16 -16 32 32" aria-hidden="true">
      {lit.halo === 0 ? null : <circle className="armada-star__halo" r={Math.min(lit.halo, 14)} />}
      {session ? <rect className="armada-star__core" x={-r} y={-r} width={r * 2} height={r * 2} transform="rotate(45)" opacity={lit.alpha} /> : <circle className="armada-star__core" r={r} opacity={lit.alpha} />}
      {pods.length === 0 ? null : <circle className="armada-ship__orbit-path" r={12} />}
      {pods.map((pod, at) => (
        <g key={pod.key} className="armada-ship__orbit" data-working={pod.working || undefined} style={{ ["--at" as string]: `${-60 + at * 180}deg` }}>
          <rect className="armada-ship__pod" data-hue={pod.hue} x={10} y={-2} width={4} height={4} transform="rotate(45 12 0)" />
        </g>
      ))}
    </svg>
  );
}

/** What a bay says: its holder's title and the line under it, or what an empty bay is. */
function said(bay: Bay): { title: string; fact: string } {
  const { holder } = bay;
  const branch = bay.branch ?? "";
  const behind = bay.behind === undefined ? undefined : bay.behind === 0 ? "level with main" : `${bay.behind} behind main`;
  switch (holder.kind) {
    case "job":
      return { title: holder.title, fact: [holder.item?.fact, branch].filter(Boolean).join(" · ") };
    case "session":
      return { title: holder.title, fact: ["Session", branch].filter(Boolean).join(" · ") };
    case "free":
      return { title: "Free", fact: ["Lit and waiting", bay.warm ? "warm" : "cold", behind].filter(Boolean).join(" · ") };
    case "closed":
      return { title: "Closed", fact: "Dark until it is opened" };
    case "other":
      return { title: holder.said, fact: branch };
  }
}

const itemOf = (bay: Bay) => (bay.holder.kind === "job" || bay.holder.kind === "session" ? bay.holder.item : undefined);

/** A short word for a card on the line: the state, beside the queue's place where it has one. */
const PULL_SAID: Record<Dot["state"], string> = { queued: "Queued", running: "Checks running", failing: "Failing", ready: "Ready", blocked: "Blocked", open: "Open" };

/** The hull's grid: its columns, every row an even share, and under the picked bay's row a drawer that says more. */
function rowsOf(bays: readonly Bay[], cols: number, selected: string | undefined): CSSProperties {
  const rows = Math.max(1, Math.ceil(bays.length / cols));
  const picked = selected === undefined ? -1 : bays.findIndex((bay) => itemOf(bay)?.key === selected);
  const row = picked === -1 ? -1 : Math.floor(picked / cols);
  const tracks = Array.from({ length: rows }, (_, at) => (at === row ? "minmax(calc(var(--space-12) * 1.75), 1.4fr) auto" : "minmax(calc(var(--space-12) + var(--space-1)), 1fr)")).join(" ");
  return { ["--cols" as string]: cols, gridTemplateRows: tracks };
}

/** How long a bay has been held, from when its holder took it. */
function heldFor(since: string | undefined): string | undefined {
  const at = since === undefined ? NaN : Date.parse(since);
  if (Number.isNaN(at)) return undefined;
  const minutes = Math.max(0, Math.round((Date.now() - at) / 60_000));
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}

const ordinal = (place: number) => `${place}${place === 1 ? "st" : place === 2 ? "nd" : place === 3 ? "rd" : "th"}`;

/** Faint dust behind the ship, as the map has: placed by a fixed hash, so it never shifts between draws. */
const DUST = Array.from({ length: 80 }, (_, at) => {
  const hash = (salt: number) => {
    const x = Math.sin(at * 127.1 + salt * 311.7) * 43758.5453;
    return x - Math.floor(x);
  };
  return { x: `${(hash(1) * 100).toFixed(2)}%`, y: `${(hash(2) * 100).toFixed(2)}%`, r: hash(3) > 0.92 ? 1.1 : 0.6 };
});

type Wire = { key: string; lit: string | undefined; kind: "out" | "in"; state: Dot["state"] | undefined; queued: boolean; d: string };

type Point = readonly [number, number];

/** Straight runs through the given corners, each corner rounded, then a curve into the last point. */
function route(corners: readonly Point[], end: Point): string {
  const [first, ...rest] = corners;
  if (first === undefined) return "";
  const r = 5;
  let d = `M${first[0]},${first[1]}`;
  rest.forEach((at, index) => {
    const before = index === 0 ? first : rest[index - 1]!;
    const after = rest[index + 1];
    if (after === undefined) return void (d += ` L${at[0]},${at[1]}`);
    const into = [Math.sign(at[0] - before[0]), Math.sign(at[1] - before[1])];
    const out = [Math.sign(after[0] - at[0]), Math.sign(after[1] - at[1])];
    d += ` L${at[0] - into[0]! * r},${at[1] - into[1]! * r} Q${at[0]},${at[1]} ${at[0] + out[0]! * r},${at[1] + out[1]! * r}`;
  });
  const last = corners[corners.length - 1]!;
  const bend = Math.max(16, Math.abs(end[0] - last[0]) / 2) * Math.sign(end[0] - last[0] || 1);
  return `${d} C${last[0] + bend},${last[1]} ${end[0] - bend},${end[1]} ${end[0]},${end[1]}`;
}

/** As many columns as the hull's width holds bays wide enough for a title, at most four: by width alone, so picking a bay never reshuffles them. */
function columnsFor(count: number, width: number, gap: number): number {
  if (count === 0 || width === 0) return 1;
  const fits = Math.floor((width + gap) / (160 + gap));
  return Math.max(1, Math.min(4, count, fits));
}

/** What the picked bay says beyond its room: where its branch stands, its step, its Sessions and pull request, and every act. */
function Drawer({ bay, col, cols, onOpenLink, onOpenSession, actsOf }: {
  bay: Bay;
  col: number;
  cols: number;
  onOpenLink: (address: string) => void;
  onOpenSession: (id: string) => void;
  actsOf: (item: Item) => ReactNode;
}) {
  const item = itemOf(bay);
  if (item === undefined) return null;
  const steps = item.steps;
  const held = heldFor(bay.since);
  const facts: [string, string][] = [
    ["Branch", [bay.branch, bay.behind === undefined ? undefined : bay.behind === 0 ? "level with main" : `${bay.behind} behind main`].filter(Boolean).join(" · ")],
    ...(held === undefined ? [] : [["Held", held] as [string, string]]),
    ...(steps === undefined || item.stepAt === undefined ? [] : [["Step", `${steps[item.stepAt]?.label ?? "—"} · ${item.stepAt + 1} of ${steps.length}`] as [string, string]]),
    ["Doing", item.fact],
    ...(bay.riders.length === 0 ? [] : [["Sessions", bay.riders.map((one) => one.session.title ?? one.session.id).join(", ")] as [string, string]]),
    ["Merge line", bay.dot === undefined ? "No pull request yet" : [bay.dot.card.heading, bay.dot.card.place === undefined ? undefined : `${ordinal(bay.dot.card.place)} in line`, bay.dot.card.says].filter(Boolean).join(" · ")],
  ];
  const link = bay.dot?.act?.kind === "link" ? bay.dot.act.url : undefined;
  return (
    <li className="armada-ship__drawer" role="group" aria-label={`Bay ${bay.slot} in full`} style={{ ["--notch" as string]: `${((col + 0.5) / cols) * 100}%` }} onClick={(event) => event.stopPropagation()}>
      <dl className="armada-ship__facts">
        {facts.map(([term, value]) => (
          <div key={term}>
            <dt>{term}</dt>
            <dd title={value}>{value}</dd>
          </div>
        ))}
      </dl>
      <span className="armada-ship__acts">
        {actsOf(item)}
        {link === undefined || bay.dot === undefined ? null : (
          <Button variant="ghost" size="sm" onClick={() => onOpenLink(link)}>
            Open {bay.dot.card.heading}
          </Button>
        )}
        {bay.riders.slice(0, 2).map((one) => (
          <Button key={one.session.id} variant="ghost" size="sm" onClick={() => onOpenSession(one.session.id)}>
            Open {one.session.title ?? "Session"}
          </Button>
        ))}
      </span>
    </li>
  );
}

export function Bays({ read, reading, selected, onSelect, onOpenLink, onOpenSession, actsOf }: {
  read: BaysRead;
  /** Fleet has not answered for the pool yet. */
  reading: boolean;
  /** The key of the tile picked, a Job's or a Session's. */
  selected: string | undefined;
  onSelect: (key: string) => void;
  onOpenLink: (address: string) => void;
  onOpenSession: (id: string) => void;
  /** The picked tile's acts, drawn in its bay. */
  actsOf: (item: Item) => ReactNode;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [wires, setWires] = useState<readonly Wire[]>([]);
  const [hovered, setHovered] = useState<string>();
  const [columns, setColumns] = useState<Readonly<Record<string, number>>>({});
  const bays = read.harbours.flatMap((one) => one.bays);
  const own = selected === undefined ? undefined : (bays.find((bay) => itemOf(bay)?.key === selected)?.key ?? read.waiting.find((one) => one.key === selected)?.key);
  // Only a pointer lights a bay and steps the rest back; the cursor marks its bay and wire alone.
  const lit = hovered;
  const shown = (key: string | undefined) => key !== undefined && (key === hovered || key === own);
  // Each repository's next one in goes to that repository's first free bay; a bay never takes another's work.
  const berths = new Map(
    read.harbours.flatMap((harbour) => {
      const free = harbour.bays.find((bay) => bay.holder.kind === "free");
      const head = read.waiting.find((item) => item.job?.owner_manifest_id === harbour.manifest);
      return free === undefined || head === undefined ? [] : [[head.key, free] as const];
    }),
  );
  const berthed = new Set([...berths.values()].map((bay) => bay.key));

  // Wires join boxes in three columns the browser lays out, so they are drawn from where it put them.
  useLayoutEffect(() => {
    const root = host.current;
    if (root === null) return;
    const find = (selector: string) => root.querySelector(selector)?.getBoundingClientRect();
    const draw = () => {
      const base = root.getBoundingClientRect();
      const x = (value: number) => value - base.left;
      const y = (value: number) => value - base.top;
      const out = bays.flatMap((bay): Wire[] => {
        if (bay.dot === undefined) return [];
        const room = root.querySelector<HTMLElement>(`[data-bay="${CSS.escape(bay.key)}"]`);
        const grid = room?.parentElement?.getBoundingClientRect();
        const card = find(`[data-card="${CSS.escape(bay.dot.key)}"]`);
        if (room === null || room === undefined || grid === undefined || card === undefined) return [];
        const box = room.getBoundingClientRect();
        const col = Number(room.dataset.col);
        const cols = Number(room.dataset.cols);
        const port: Point = [x(box.right), y(box.top + box.height / 2)];
        const into: Point = [x(card.left), y(card.top + card.height / 2)];
        // The rightmost column goes straight out; any other runs down the gap to its right, then along the
        // gap below its row to the hull's wall, each column at its own height in that gap.
        const gapX = (Number(getComputedStyle(room.parentElement!).columnGap.replace("px", "")) || 16) / 2;
        const gapY = (Number(getComputedStyle(room.parentElement!).rowGap.replace("px", "")) || 12) / 2;
        const lane = (col - (cols - 2) / 2) * 3;
        const corners: Point[] =
          col === cols - 1
            ? [port]
            : [port, [port[0] + gapX, port[1]], [port[0] + gapX, y(box.bottom) + gapY + lane], [x(grid.right) + gapX, y(box.bottom) + gapY + lane]];
        return [{ key: `out:${bay.key}`, lit: bay.key, kind: "out", state: bay.dot.state, queued: bay.dot.queued, d: route(corners, into) }];
      });
      const inbound = [...berths].flatMap(([key, bay]): Wire[] => {
        const from = find(`[data-wait="${CSS.escape(key)}"]`);
        const room = root.querySelector<HTMLElement>(`[data-bay="${CSS.escape(bay.key)}"]`);
        const grid = room?.parentElement?.getBoundingClientRect();
        if (from === undefined || room === null || room === undefined || grid === undefined) return [];
        const box = room.getBoundingClientRect();
        const gapX = (Number(getComputedStyle(room.parentElement!).columnGap.replace("px", "")) || 16) / 2;
        const gapY = (Number(getComputedStyle(room.parentElement!).rowGap.replace("px", "")) || 12) / 2;
        const start: Point = [x(from.right), y(from.top + from.height / 2)];
        const door: Point = [x(box.left), y(box.top + box.height / 2)];
        // Into the first column straight; deeper in, along the gap above the bay's row and down the gap to its left.
        if (Number(room.dataset.col) === 0) return [{ key: `in:${key}`, lit: key, kind: "in", state: undefined, queued: false, d: route([start], door) }];
        const above = y(box.top) - gapY;
        const corners: Point[] = [[x(grid.left) - gapX, above], [x(box.left) - gapX, above], [x(box.left) - gapX, door[1]], door];
        const lead = route([start], corners[0]!);
        const run = route(corners.slice(0, -1), door).replace(/^M[^ ]+/, "");
        return [{ key: `in:${key}`, lit: key, kind: "in", state: undefined, queued: false, d: `${lead}${run}` }];
      });
      const drawn = [...out, ...inbound];
      setWires((was) => (JSON.stringify(was) === JSON.stringify(drawn) ? was : drawn));
      // Each hull's columns, from the room its grid has.
      const next: Record<string, number> = {};
      root.querySelectorAll<HTMLElement>(".armada-ship__bays").forEach((grid) => {
        const count = grid.querySelectorAll(":scope > .armada-ship__bay").length;
        next[grid.dataset.harbour ?? ""] = columnsFor(count, grid.clientWidth, Number(getComputedStyle(grid).columnGap.replace("px", "")) || 16);
      });
      setColumns((was) => (JSON.stringify(was) === JSON.stringify(next) ? was : next));
    };
    draw();
    const watch = new ResizeObserver(draw);
    watch.observe(root);
    root.querySelectorAll(".armada-ship__bays").forEach((grid) => watch.observe(grid));
    // A scrolled list moves what a wire ends at, so a scroll draws again.
    const scrollers = [...root.querySelectorAll(".armada-ship__cards, .armada-ship__fleet")];
    scrollers.forEach((one) => one.addEventListener("scroll", draw));
    return () => {
      watch.disconnect();
      scrollers.forEach((one) => one.removeEventListener("scroll", draw));
    };
  }, [read, columns, selected]);

  if (read.harbours.length === 0) {
    return <p className="armada-ship__empty">{reading ? "Reading the bays…" : "Fleet serves no worktree bays for this repository."}</p>;
  }
  const named = read.harbours.length > 1;
  const wired = new Map(bays.flatMap((bay) => (bay.dot === undefined ? [] : [[bay.dot.key, bay.key] as const])));
  return (
    <div ref={host} className="armada-ship" data-lighting={lit === undefined ? undefined : ""} onMouseLeave={() => setHovered(undefined)}>
      <svg className="armada-ship__dust" aria-hidden="true">
        {DUST.map((one, at) => (
          <circle key={at} className="armada-map__dust" cx={one.x} cy={one.y} r={one.r} />
        ))}
      </svg>
      <svg className="armada-ship__wires" aria-hidden="true">
        <defs>
          <marker id="armada-ship-in" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
            <path d="M0,0 L8,4 L0,8 z" className="armada-ship__arrow" />
          </marker>
        </defs>
        {wires.map((one) => (
          <path
            key={one.key}
            className="armada-ship__wire"
            d={one.d}
            data-kind={one.kind}
            data-state={one.state}
            data-queued={one.queued || undefined}
            data-lit={shown(one.lit) || undefined}
            markerEnd={one.kind === "in" ? "url(#armada-ship-in)" : undefined}
          />
        ))}
      </svg>

      <aside className="armada-ship__queue" aria-label="Waiting for a bay">
        <h4 className="armada-ship__head">
          Waiting for a bay <span>{read.waiting.length}</span>
        </h4>
        {read.waiting.length === 0 ? (
          <p className="armada-ship__quiet">Nothing waiting</p>
        ) : (
          <ol className="armada-ship__waiting">
            {read.waiting.map((item, at) => (
              <li key={item.key}>
                <button
                  type="button"
                  className="armada-ship__wait"
                  data-wait={item.key}
                  data-next={berths.has(item.key) || undefined}
                  data-hue={item.hue}
                  data-lit={lit === item.key || undefined}
                  aria-pressed={item.key === selected}
                  onMouseEnter={() => setHovered(item.key)}
                  onClick={() => onSelect(item.key)}
                >
                  <Mark item={item} session={item.session === true} />
                  <span className="armada-ship__what">
                    <span className="armada-ship__title">{item.title}</span>
                    <span className="armada-ship__fact">{berths.has(item.key) ? `Next in, to bay ${String(berths.get(item.key)!.slot).padStart(2, "0")}` : at === 0 ? "Next in, when a bay frees" : `${ordinal(at + 1)} in line`}</span>
                  </span>
                </button>
              </li>
            ))}
          </ol>
        )}
        {read.adrift.length === 0 ? null : (
          <>
            <h4 className="armada-ship__head">Not in a bay</h4>
            <ul className="armada-ship__adrift">
              {read.adrift.map((item) => (
                <li key={item.key}>
                  <button type="button" className="armada-ship__drifter" data-hue={item.hue} aria-pressed={item.key === selected} onClick={() => onSelect(item.key)}>
                    <Mark item={item} session={item.session === true} />
                    <span>{item.title}</span>
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
      </aside>

      <div className="armada-ship__fleet">
        {read.harbours.map((harbour) => {
          const held = harbour.bays.filter((bay) => bay.holder.kind === "job" || bay.holder.kind === "session").length;
          const working = harbour.bays.some((bay) => itemOf(bay)?.hue === "running");
          return (
            <section key={harbour.manifest} className="armada-ship__hull" aria-label={harbour.name} style={{ flexGrow: Math.ceil(harbour.bays.length / (columns[harbour.manifest] ?? 2)) }} data-working={working || undefined}>
              <header className="armada-ship__bow">
                <span className="armada-ship__name">{harbour.name}</span>
                <span className="armada-ship__fact">
                  {held} of {harbour.bays.length} bays held
                </span>
              </header>
              <ol className="armada-ship__bays" role="listbox" aria-label="Bays" data-harbour={harbour.manifest} style={rowsOf(harbour.bays, columns[harbour.manifest] ?? 2, selected)}>
                {harbour.bays.flatMap((bay, index) => {
                  const item = itemOf(bay);
                  const { title, fact } = said(bay);
                  const on = item !== undefined && item.key === selected;
                  const cols = columns[harbour.manifest] ?? 2;
                  const room = (
                    <li
                      key={bay.key}
                      className="armada-ship__bay"
                      role="option"
                      aria-selected={on}
                      aria-label={`Bay ${bay.slot}: ${title}`}
                      data-bay={bay.key}
                      data-col={index % cols}
                      data-cols={cols}
                      data-kind={bay.holder.kind}
                      data-hue={item?.hue}
                      data-lit={lit === bay.key || undefined}
                      data-berth={berthed.has(bay.key) || undefined}
                      onMouseEnter={() => setHovered(bay.key)}
                      onClick={() => (item === undefined ? undefined : onSelect(item.key))}
                    >
                      <span className="armada-ship__bay-top">
                        <span className="armada-ship__lamp" aria-hidden="true" />
                        <span className="armada-ship__bay-number">{String(bay.slot).padStart(2, "0")}</span>
                        {bay.dot === undefined ? null : (
                          <span className="armada-ship__badge" data-state={bay.dot.state}>
                            {bay.dot.card.heading.startsWith("#") ? bay.dot.card.heading : "Landing"}
                            {bay.dot.card.place === undefined ? "" : ` · ${ordinal(bay.dot.card.place)}`}
                          </span>
                        )}
                      </span>
                      <span className="armada-ship__room">
                        {bay.holder.kind === "job" || bay.holder.kind === "session" ? (
                          <Mark item={item} session={bay.holder.kind === "session"} pods={bay.riders.map((one) => ({ key: one.session.id, hue: one.item?.hue ?? "queued", working: one.session.turn.state === "working" }))} />
                        ) : (
                          <span className="armada-ship__floor" aria-hidden="true" />
                        )}
                        <span className="armada-ship__what">
                          <span className="armada-ship__title">{title}</span>
                          <span className="armada-ship__fact">{fact}</span>
                        </span>
                      </span>
                      <span className="armada-ship__port" data-wired={bay.dot === undefined ? undefined : ""} aria-hidden="true" />
                    </li>
                  );
                  // The drawer opens under the picked bay's row, its notch over the bay.
                  const cols2 = columns[harbour.manifest] ?? 2;
                  const picked = selected === undefined ? -1 : harbour.bays.findIndex((one) => itemOf(one)?.key === selected);
                  const endOfRow = picked === -1 ? -1 : Math.min(harbour.bays.length - 1, Math.floor(picked / cols2) * cols2 + cols2 - 1);
                  if (index !== endOfRow) return [room];
                  const chosen = harbour.bays[picked]!;
                  return [room, <Drawer key={`drawer:${chosen.key}`} bay={chosen} col={picked % cols2} cols={cols2} onOpenLink={onOpenLink} onOpenSession={onOpenSession} actsOf={actsOf} />];
                })}
              </ol>
              <span className="armada-ship__stern" aria-hidden="true">
                <span className="armada-ship__thruster" />
                <span className="armada-ship__thruster" />
              </span>
            </section>
          );
        })}
      </div>

      <aside className="armada-ship__line" aria-label="Merge line">
        <h4 className="armada-ship__head">Merge line</h4>
        <div className="armada-ship__cards">
          {read.harbours.map((harbour) => (
            <ol key={harbour.manifest} className="armada-ship__deck" aria-label={named ? `${harbour.name} merge line` : "Merge line"}>
              {named ? <li className="armada-ship__deck-name">{harbour.name}</li> : null}
              {harbour.line.map((dot) => {
                const bay = wired.get(dot.key);
                const act = dot.act;
                const pull = dot.card.heading.startsWith("#");
                return (
                  <li key={dot.key}>
                    <Tooltip label={dot.tip}>
                      <button
                        type="button"
                        className="armada-ship__card"
                        data-card={dot.key}
                        data-state={dot.state}
                        data-queued={dot.queued || undefined}
                        data-wired={bay === undefined ? undefined : ""}
                        data-lit={(bay !== undefined && bay === lit) || undefined}
                        onMouseEnter={() => setHovered(bay)}
                        onClick={() => (act?.kind === "link" ? onOpenLink(act.url) : undefined)}
                      >
                        <span className="armada-ship__card-head">
                          <span className="armada-ship__number">{pull ? dot.card.heading : "Landing"}</span>
                          {dot.card.place === undefined ? null : <span className="armada-ship__place">{ordinal(dot.card.place)}</span>}
                          <span className="armada-ship__state">{PULL_SAID[dot.state]}</span>
                        </span>
                        <span className="armada-ship__card-title">{dot.card.title ?? dot.card.branch}</span>
                      </button>
                    </Tooltip>
                  </li>
                );
              })}
              <li className="armada-ship__main" data-red={harbour.main === "red" || undefined}>
                <span className="armada-view__main" data-red={harbour.main === "red" || undefined} />
                <span>main</span>
                <span className="armada-ship__state">{harbour.main === "red" ? "red" : harbour.main === "green" ? "green" : ""}</span>
              </li>
            </ol>
          ))}
        </div>
      </aside>
    </div>
  );
}

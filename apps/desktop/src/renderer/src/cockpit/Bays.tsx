// The Cockpit's Bays: each repository a ship, its worktree bays the compartments down its hull, the work
// in them marked as the map marks it (a Job a star, a Session a diamond, lit by how much it wants the
// owner) with the Sessions riding with a Job in orbit round it. What waits for a bay queues at the
// hatches on the left; on the right the merge line stands as cards, and each bay with a pull request is
// wired out to its card, main at the foot. A free bay is empty with its lamp lit; a closed one is dark.
// What moves is what is still working: a working Session's orbit, a wire whose checks run.

import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { Tooltip } from "@armada/components";

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

/** A curve from the right edge of one box to the left edge of another, both read where the browser put them. */
function curve(base: DOMRect, from: DOMRect, to: DOMRect): string {
  const x1 = from.right - base.left;
  const y1 = from.top + from.height / 2 - base.top;
  const x2 = to.left - base.left;
  const y2 = to.top + to.height / 2 - base.top;
  const bend = Math.max(24, (x2 - x1) / 2);
  return `M${x1},${y1} C${x1 + bend},${y1} ${x2 - bend},${y2} ${x2},${y2}`;
}

export function Bays({ read, reading, selected, onSelect, onOpenLink, actsOf }: {
  read: BaysRead;
  /** Fleet has not answered for the pool yet. */
  reading: boolean;
  /** The key of the tile picked, a Job's or a Session's. */
  selected: string | undefined;
  onSelect: (key: string) => void;
  onOpenLink: (address: string) => void;
  /** The picked tile's acts, drawn in its bay. */
  actsOf: (item: Item) => ReactNode;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [wires, setWires] = useState<readonly Wire[]>([]);
  const [hovered, setHovered] = useState<string>();
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
      const out = bays.flatMap((bay): Wire[] => {
        if (bay.dot === undefined) return [];
        const from = find(`[data-port="${CSS.escape(bay.key)}"]`);
        const to = find(`[data-card="${CSS.escape(bay.dot.key)}"]`);
        return from === undefined || to === undefined ? [] : [{ key: `out:${bay.key}`, lit: bay.key, kind: "out", state: bay.dot.state, queued: bay.dot.queued, d: curve(base, from, to) }];
      });
      const inbound = [...berths].flatMap(([key, bay]): Wire[] => {
        const from = find(`[data-wait="${CSS.escape(key)}"]`);
        const to = find(`[data-hatch="${CSS.escape(bay.key)}"]`);
        return from === undefined || to === undefined ? [] : [{ key: `in:${key}`, lit: key, kind: "in", state: undefined, queued: false, d: curve(base, from, to) }];
      });
      const drawn = [...out, ...inbound];
      setWires((was) => (JSON.stringify(was) === JSON.stringify(drawn) ? was : drawn));
    };
    draw();
    const watch = new ResizeObserver(draw);
    watch.observe(root);
    const cards = root.querySelector(".armada-ship__cards");
    cards?.addEventListener("scroll", draw);
    return () => {
      watch.disconnect();
      cards?.removeEventListener("scroll", draw);
    };
  }, [read]);

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
            <section key={harbour.manifest} className="armada-ship__hull" aria-label={harbour.name} style={{ flexGrow: harbour.bays.length }} data-working={working || undefined}>
              <header className="armada-ship__bow">
                <span className="armada-ship__name">{harbour.name}</span>
                <span className="armada-ship__fact">
                  {held} of {harbour.bays.length} bays held
                </span>
              </header>
              <ol className="armada-ship__bays" role="listbox" aria-label="Bays">
                {harbour.bays.map((bay) => {
                  const item = itemOf(bay);
                  const { title, fact } = said(bay);
                  const on = item !== undefined && item.key === selected;
                  const steps = item?.steps;
                  return (
                    <li
                      key={bay.key}
                      className="armada-ship__bay"
                      role="option"
                      aria-selected={on}
                      aria-label={`Bay ${bay.slot}: ${title}`}
                      data-kind={bay.holder.kind}
                      data-hue={item?.hue}
                      data-lit={lit === bay.key || undefined}
                      data-berth={berthed.has(bay.key) || undefined}
                      onMouseEnter={() => setHovered(bay.key)}
                      onClick={() => (item === undefined ? undefined : onSelect(item.key))}
                    >
                      <span className="armada-ship__hatch" data-hatch={bay.key} aria-hidden="true">
                        <span className="armada-ship__lamp" />
                        {String(bay.slot).padStart(2, "0")}
                      </span>
                      {bay.holder.kind === "job" || bay.holder.kind === "session" ? (
                        <Mark item={item} session={bay.holder.kind === "session"} pods={bay.riders.map((one) => ({ key: one.session.id, hue: one.item?.hue ?? "queued", working: one.session.turn.state === "working" }))} />
                      ) : (
                        <span className="armada-ship__floor" aria-hidden="true" />
                      )}
                      <span className="armada-ship__what">
                        <span className="armada-ship__title">{title}</span>
                        <span className="armada-ship__fact">{fact}</span>
                      </span>
                      {bay.riders.length === 0 ? null : (
                        <span className="armada-ship__riders">
                          {bay.riders.slice(0, 2).map((one) => (
                            <Tooltip key={one.session.id} label={`Session: ${one.session.title ?? one.session.id}`}>
                              <button
                                type="button"
                                className="armada-ship__drifter"
                                data-hue={one.item?.hue ?? "queued"}
                                onClick={(event) => {
                                  event.stopPropagation();
                                  if (one.item !== undefined) onSelect(one.item.key);
                                }}
                              >
                                <Mark item={one.item} session />
                                <span>{one.session.title ?? one.session.id}</span>
                              </button>
                            </Tooltip>
                          ))}
                        </span>
                      )}
                      {on && item !== undefined ? (
                        <span className="armada-ship__acts" onClick={(event) => event.stopPropagation()}>
                          {actsOf(item)}
                        </span>
                      ) : steps === undefined || steps.length < 2 ? null : (
                        <ol className="armada-ship__pips" aria-label="Steps">
                          {steps.map((step, at) => (
                            <li key={step.id} data-pip={at < (item?.stepAt ?? 0) ? "done" : at === item?.stepAt ? "live" : "ahead"} title={step.label} />
                          ))}
                        </ol>
                      )}
                      <span className="armada-ship__port" data-port={bay.key} data-wired={bay.dot === undefined ? undefined : ""} aria-hidden="true" />
                    </li>
                  );
                })}
              </ol>
              <footer className="armada-ship__stern" aria-hidden="true">
                <span className="armada-ship__thruster" />
                <span className="armada-ship__thruster" />
              </footer>
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

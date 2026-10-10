// The bays as the glass's third face, drawn as the map's sky: each worktree bay a port on the
// station's spine, its holder docked there as the map marks it (a Job a star, a Session a diamond,
// lit by how much it wants the owner) with the Sessions riding with the Job in orbit round it, and a
// burn from the bay to its pull request on the merge line, which falls through its rings to main.
// What moves is what is still working: a working Session's orbit, and a burn whose checks run.
// A press selects the holder as the grid and the map do.

import { useLayoutEffect, useRef, useState } from "react";
import { Tooltip } from "@armada/components";

import type { Item } from "../Dashboard";
import type { Bay, BaysRead, Harbour } from "./bays";
import { burn } from "./FleetMap";
import type { Dot } from "./horizon";

/** A Session in orbit round the Job it works on: its hue, and whether it is mid-turn. */
type Pod = { key: string; hue: string; working: boolean };

/** A Job's star or a Session's diamond, the map's own marks at the size of a row, its riders in orbit. */
function Mark({ item, session, pods = [] }: { item: Item | undefined; session: boolean; pods?: readonly Pod[] }) {
  const lit = item === undefined ? { r: 4.5, halo: 0, alpha: 0.75 } : burn(item);
  const r = Math.min(lit.r, 7);
  return (
    <svg className="armada-bays__mark armada-star" data-hue={item?.hue ?? "queued"} viewBox="-16 -16 32 32" aria-hidden="true">
      {lit.halo === 0 ? null : <circle className="armada-star__halo" r={Math.min(lit.halo, 14)} />}
      {session ? <rect className="armada-star__core" x={-r} y={-r} width={r * 2} height={r * 2} transform="rotate(45)" opacity={lit.alpha} /> : <circle className="armada-star__core" r={r} opacity={lit.alpha} />}
      {pods.length === 0 ? null : <circle className="armada-bays__orbit-path" r={12} />}
      {pods.map((pod, at) => (
        <g key={pod.key} className="armada-bays__orbit" data-working={pod.working || undefined} style={{ ["--at" as string]: `${-60 + at * 180}deg` }}>
          <rect className="armada-bays__pod" data-hue={pod.hue} x={10} y={-2} width={4} height={4} transform="rotate(45 12 0)" />
        </g>
      ))}
    </svg>
  );
}

/** Faint dust behind the bays, as the map has: placed by a fixed hash, so it never shifts between draws. */
const DUST = Array.from({ length: 90 }, (_, at) => {
  const hash = (salt: number) => {
    const x = Math.sin(at * 127.1 + salt * 311.7) * 43758.5453;
    return x - Math.floor(x);
  };
  return { x: `${(hash(1) * 100).toFixed(2)}%`, y: `${(hash(2) * 100).toFixed(2)}%`, r: hash(3) > 0.92 ? 1.1 : 0.6 };
});

/** The merge line's rings, widening as it falls toward main: one between each pair of dots, at most eight. */
const ringsFor = (dots: number) => {
  const count = Math.max(2, Math.min(8, dots));
  return Array.from({ length: count }, (_, at) => 8 + (at * 72) / Math.max(1, count - 1));
};

/** The holder's title and the one line under it. */
function said(bay: Bay): { title: string; fact: string } {
  const { holder } = bay;
  const branch = bay.branch ?? "";
  switch (holder.kind) {
    case "job":
      return { title: holder.title, fact: [holder.item?.fact, branch].filter(Boolean).join(" · ") };
    case "session":
      return { title: holder.title, fact: ["Session", branch].filter(Boolean).join(" · ") };
    case "free":
      return { title: "Free", fact: [bay.warm ? "warm" : "cold", bay.behind === undefined ? undefined : bay.behind === 0 ? "level with main" : `${bay.behind} behind main`].filter(Boolean).join(" · ") };
    case "closed":
      return { title: "Closed", fact: "Never leased until it is opened" };
    case "other":
      return { title: holder.said, fact: branch };
  }
}

const itemOf = (bay: Bay) => (bay.holder.kind === "job" || bay.holder.kind === "session" ? bay.holder.item : undefined);

type Thread = { key: string; bay: string; state: Dot["state"]; queued: boolean; d: string };

function HarbourField({ harbour, named, selected, lit, onLight, onSelect, onOpenLink }: {
  harbour: Harbour;
  named: boolean;
  selected: string | undefined;
  lit: string | undefined;
  onLight: (bay: string | undefined) => void;
  onSelect: (key: string) => void;
  onOpenLink: (address: string) => void;
}) {
  const field = useRef<HTMLDivElement>(null);
  const [threads, setThreads] = useState<readonly Thread[]>([]);
  // Threads join two lists the browser lays out, so they are drawn from where it put them.
  useLayoutEffect(() => {
    const host = field.current;
    if (host === null) return;
    const draw = () => {
      const base = host.getBoundingClientRect();
      const next = harbour.bays.flatMap((bay): Thread[] => {
        if (bay.dot === undefined) return [];
        const from = host.querySelector(`[data-anchor="${CSS.escape(bay.key)}"]`)?.getBoundingClientRect();
        const to = host.querySelector(`[data-node="${CSS.escape(bay.dot.key)}"]`)?.getBoundingClientRect();
        if (from === undefined || to === undefined) return [];
        const x1 = from.right - base.left;
        const y1 = from.top + from.height / 2 - base.top;
        const x2 = to.left - base.left;
        const y2 = to.top + to.height / 2 - base.top;
        const bend = (x2 - x1) / 2;
        return [{ key: bay.dot.key, bay: bay.key, state: bay.dot.state, queued: bay.dot.queued, d: `M${x1},${y1} C${x1 + bend},${y1} ${x2 - bend},${y2} ${x2},${y2}` }];
      });
      setThreads((was) => (JSON.stringify(was) === JSON.stringify(next) ? was : next));
    };
    draw();
    const watch = new ResizeObserver(draw);
    watch.observe(host);
    return () => watch.disconnect();
  }, [harbour]);

  const owned = new Map(harbour.bays.flatMap((bay) => (bay.dot === undefined ? [] : [[bay.dot.key, bay.key] as const])));
  return (
    <section className="armada-bays__harbour" aria-label={named ? harbour.name : "Bays"}>
      {named ? <h3 className="armada-bays__name">{harbour.name}</h3> : null}
      <div ref={field} className="armada-bays__field">
        <svg className="armada-bays__threads" aria-hidden="true">
          {threads.map((one) => (
            <path key={one.key} className="armada-bays__thread" d={one.d} data-state={one.state} data-queued={one.queued || undefined} data-lit={lit === one.bay || undefined} />
          ))}
        </svg>
        <ol className="armada-bays__rows" role="listbox" aria-label="Bays">
          {harbour.bays.map((bay) => {
            const item = itemOf(bay);
            const { title, fact } = said(bay);
            const on = item !== undefined && item.key === selected;
            const riders = bay.riders.slice(0, 2);
            return (
              <li
                key={bay.key}
                className="armada-bays__row"
                role="option"
                aria-selected={on}
                aria-label={`Bay ${bay.slot}: ${title}`}
                data-kind={bay.holder.kind}
                data-hue={item?.hue}
                data-lit={lit === bay.key || undefined}
                onMouseEnter={() => onLight(bay.key)}
                onClick={() => (item === undefined ? undefined : onSelect(item.key))}
              >
                <span className="armada-bays__port" aria-hidden="true">{String(bay.slot).padStart(2, "0")}</span>
                {bay.holder.kind === "free" || bay.holder.kind === "closed" || bay.holder.kind === "other" ? (
                  <span className="armada-bays__berth" aria-hidden="true" />
                ) : (
                  <Mark item={item} session={bay.holder.kind === "session"} pods={bay.riders.map((one) => ({ key: one.session.id, hue: one.item?.hue ?? "queued", working: one.session.turn.state === "working" }))} />
                )}
                <span className="armada-bays__what">
                  <span className="armada-bays__title">{title}</span>
                  <span className="armada-bays__fact">{fact}</span>
                </span>
                {riders.length === 0 ? null : (
                  <span className="armada-bays__riders">
                    {riders.map((one) => (
                      <Tooltip key={one.session.id} label={`Session: ${one.session.title ?? one.session.id}`}>
                        <button
                          type="button"
                          className="armada-bays__rider"
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
                <span className="armada-bays__pull" data-anchor={bay.key} data-none={bay.dot === undefined || undefined}>
                  {bay.dot === undefined ? (bay.holder.kind === "job" || bay.holder.kind === "session" ? "No pull request" : null) : bay.dot.card.heading.startsWith("#") ? bay.dot.card.heading : `Landing, place ${bay.dot.card.place ?? "?"}`}
                </span>
              </li>
            );
          })}
        </ol>
        <div className="armada-bays__lane">
        <svg className="armada-bays__tunnel" aria-hidden="true">
          {ringsFor(harbour.line.length).map((y, at) => (
            <ellipse key={y} cx={8} cy={`${y}%`} rx={10 + at * 1.5} ry={2.5 + at * 0.3} />
          ))}
        </svg>
        <ol className="armada-bays__line" aria-label="Merge line">
          {harbour.line.map((dot) => {
            const act = dot.act;
            return (
              <li key={dot.key} className="armada-bays__node" data-lit={owned.get(dot.key) === lit || undefined} data-bayed={owned.has(dot.key) || undefined} onMouseEnter={() => onLight(owned.get(dot.key))}>
                <Tooltip label={dot.tip}>
                  <button type="button" className="armada-view__dot" data-node={dot.key} data-state={dot.state} data-queued={dot.queued || undefined} aria-label={dot.tip} onClick={() => (act?.kind === "link" ? onOpenLink(act.url) : undefined)} />
                </Tooltip>
                <span className="armada-bays__node-said">
                  <span>{dot.card.heading.startsWith("#") ? dot.card.heading : dot.card.branch}</span>
                  {dot.card.place === undefined ? null : <span className="armada-bays__place">{dot.card.place}</span>}
                </span>
              </li>
            );
          })}
          <li className="armada-bays__node armada-bays__main" data-red={harbour.main === "red" || undefined}>
            <span className="armada-bays__planet" aria-hidden="true" />
            <span className="armada-view__main" data-red={harbour.main === "red" || undefined} />
            <span className="armada-bays__node-said">
              <span>main</span>
            </span>
          </li>
        </ol>
        </div>
      </div>
    </section>
  );
}

export function Bays({ read, reading, selected, onSelect, onOpenLink }: {
  read: BaysRead;
  /** Fleet has not answered for the pool yet. */
  reading: boolean;
  /** The key of the tile picked, a Job's or a Session's. */
  selected: string | undefined;
  onSelect: (key: string) => void;
  onOpenLink: (address: string) => void;
}) {
  const own = selected === undefined ? undefined : read.harbours.flatMap((one) => one.bays).find((bay) => itemOf(bay)?.key === selected)?.key;
  const [hovered, setHovered] = useState<string>();
  const lit = hovered ?? own;
  if (read.harbours.length === 0) {
    return <p className="armada-bays__empty">{reading ? "Reading the bays…" : "Fleet serves no worktree bays for this repository."}</p>;
  }
  return (
    <div className="armada-bays" data-lighting={lit === undefined ? undefined : ""} onMouseLeave={() => setHovered(undefined)}>
      <svg className="armada-bays__dust" aria-hidden="true">
        {DUST.map((one, at) => (
          <circle key={at} className="armada-map__dust" cx={one.x} cy={one.y} r={one.r} />
        ))}
      </svg>
      {read.harbours.map((harbour) => (
        <HarbourField key={harbour.manifest} harbour={harbour} named={read.harbours.length > 1} selected={selected} lit={lit} onLight={setHovered} onSelect={onSelect} onOpenLink={onOpenLink} />
      ))}
      {read.unbayed.length === 0 ? null : (
        <footer className="armada-bays__unbayed" aria-label="Without a bay">
          <span className="armada-bays__fact">Without a bay</span>
          {read.unbayed.map((item) => (
            <button key={item.key} type="button" className="armada-bays__rider" data-hue={item.hue} aria-pressed={item.key === selected} onClick={() => onSelect(item.key)}>
              <Mark item={item} session={item.session === true} />
              <span>{item.title}</span>
            </button>
          ))}
        </footer>
      )}
    </div>
  );
}

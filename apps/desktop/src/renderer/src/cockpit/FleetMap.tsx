// The fleet as a map: each Job and Session a star, its colour and glow from how it stands (the same
// reading the tile's glyph names), the one that needs the owner the brightest. Lines join what is
// connected; repositories are faint clusters. It is the grid's other face and offers the same acts:
// a press or the arrows select, and the keys the grid answers answer here. Nothing moves, and
// nothing pulses. Mock only, as the Dashboard is.

import { useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";

import type { Item } from "../Dashboard";
import type { Field, Link, Sky } from "./map-layout";

const LINE_SAID: Record<Link["kind"], string> = { dispatched: "Dispatched", waits: "Waits on", works: "Holds", line: "On the merge line" };

/** How bright a star burns: what needs the owner most, what is over least. */
export function burn(item: Item): { r: number; halo: number; alpha: number } {
  if (item.hue === "ask" || item.hue === "issue") return { r: 9, halo: 30, alpha: 1 };
  if (item.hue === "running") return { r: 6, halo: 15, alpha: 0.95 };
  if (item.hue === "queued") return { r: 4.5, halo: 0, alpha: 0.75 };
  return { r: 5, halo: 10, alpha: 0.65 };
}

const short = (title: string) => (title.length > 18 ? `${title.slice(0, 17)}…` : title);

export function FleetMap({
  sky,
  field,
  onField,
  selected,
  onSelect,
  onKey,
}: {
  sky: Sky;
  /** The room it was laid out in, and what it is told when that changes. */
  field: Field;
  onField: (field: Field) => void;
  selected: string | undefined;
  onSelect: (key: string) => void;
  onKey?: (event: KeyboardEvent) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const one = host.current;
    if (one === null) return;
    const read = () => {
      const box = one.getBoundingClientRect();
      if (box.width > 0 && box.height > 0 && (Math.round(box.width) !== field.width || Math.round(box.height) !== field.height)) onField({ width: Math.round(box.width), height: Math.round(box.height) });
    };
    read();
    const watch = new ResizeObserver(read);
    watch.observe(one);
    return () => watch.disconnect();
  }, [field.width, field.height]);
  const [tip, setTip] = useState<{ said: string; title: string; left: number; top: number }>();
  const place = new Map<string, { x: number; y: number }>([...sky.stars.map((s) => [s.key, s] as const), ...sky.stars.map((s) => [s.item.job?.id ?? s.key, s] as const), ...sky.landmarks.map((m) => [m.key, m] as const)]);

  return (
    <div ref={host} className="armada-map" data-selected={selected}>
      <svg className="armada-map__field" viewBox={`0 0 ${field.width} ${field.height}`} role="listbox" aria-label="Map" tabIndex={0} onKeyDown={onKey}>
        {sky.dust.map((one, index) => (
          <circle key={index} className="armada-map__dust" cx={one.x} cy={one.y} r={one.r} />
        ))}
        {sky.clusters.map((one) => (
          <g key={one.id} className="armada-map__cluster" aria-hidden="true">
            <circle cx={one.x} cy={one.y} r={one.r} />
            <text x={one.x} y={one.y - one.r - 8} textAnchor="middle">
              {one.name}
            </text>
          </g>
        ))}
        {sky.links.map((one, index) => {
          const a = place.get(one.from);
          const b = place.get(one.to);
          if (a === undefined || b === undefined) return null;
          return <line key={index} className="armada-map__link" data-kind={one.kind} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
        })}
        {sky.landmarks.map((one) => (
          <g key={one.key} className="armada-map__landmark" data-red={one.red || undefined}>
            {one.branches.map((branch) => (
              <g
                key={branch.key}
                onMouseEnter={(event) => {
                  const box = event.currentTarget.getBoundingClientRect();
                  const base = host.current?.getBoundingClientRect();
                  if (base !== undefined) setTip({ said: "On the merge line", title: branch.key, left: box.x - base.x + box.width / 2, top: box.y - base.y });
                }}
                onMouseLeave={() => setTip(undefined)}
              >
                <line className="armada-map__link" data-kind="line" x1={one.x} y1={one.y} x2={branch.x} y2={branch.y} />
                <circle className="armada-map__branch" cx={branch.x} cy={branch.y} r={3.5} />
              </g>
            ))}
            <rect className="armada-map__main" x={one.x - 7} y={one.y - 7} width={14} height={14} />
            <text x={one.x} y={one.y - 14} textAnchor="middle">
              {one.label}
            </text>
          </g>
        ))}
        {sky.stars.map((star) => {
          const { item } = star;
          const lit = burn(item);
          const session = item.session === true;
          const said = item.state ?? item.kind;
          const on = star.key === selected;
          const show = (event: { currentTarget: SVGGElement }) => {
            const box = event.currentTarget.getBoundingClientRect();
            const base = host.current?.getBoundingClientRect();
            if (base !== undefined) setTip({ said, title: item.title, left: box.x - base.x + box.width / 2, top: box.y - base.y });
          };
          return (
            <g
              key={star.key}
              className="armada-star"
              role="option"
              aria-selected={on}
              aria-label={`${item.title}, ${session ? "Session" : "Job"}, ${said}`}
              data-hue={item.hue}
              data-lit={item.hue === "ask" || item.hue === "issue" || undefined}
              data-job-id={item.job?.id}
              data-status={item.job?.status}
              onClick={() => onSelect(star.key)}
              onMouseEnter={show}
              onMouseLeave={() => setTip(undefined)}
            >
              {lit.halo === 0 ? null : <circle className="armada-star__halo" cx={star.x} cy={star.y} r={lit.halo} />}
              {lit.halo < 20 ? null : <circle className="armada-star__halo" data-inner cx={star.x} cy={star.y} r={lit.halo * 0.55} />}
              {session ? (
                <rect className="armada-star__core" x={star.x - lit.r} y={star.y - lit.r} width={lit.r * 2} height={lit.r * 2} transform={`rotate(45 ${star.x} ${star.y})`} opacity={lit.alpha} />
              ) : (
                <circle className="armada-star__core" cx={star.x} cy={star.y} r={lit.r} opacity={lit.alpha} />
              )}
              {on ? <circle className="armada-star__ring" cx={star.x} cy={star.y} r={lit.r + 8} /> : null}
              <text className="armada-star__label" x={star.x} y={star.y + lit.r + 16} textAnchor="middle">
                {short(item.title)}
              </text>
            </g>
          );
        })}
      </svg>
      {tip === undefined ? null : (
        <div className="armada-map__tip" style={{ left: tip.left, top: tip.top }} role="tooltip">
          <span>{tip.said}</span>
          {tip.title}
        </div>
      )}
      <ul className="armada-map__key" aria-label="Lines">
        {(Object.keys(LINE_SAID) as Link["kind"][])
          .filter((kind) => sky.links.some((one) => one.kind === kind))
          .map((kind) => (
            <li key={kind} data-kind={kind}>
              <svg width="20" height="6" aria-hidden="true">
                <line className="armada-map__link" data-kind={kind} x1="0" y1="3" x2="20" y2="3" />
              </svg>
              {LINE_SAID[kind]}
            </li>
          ))}
      </ul>
    </div>
  );
}

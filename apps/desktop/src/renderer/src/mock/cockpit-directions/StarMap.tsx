// The Cockpit's map, restored from `667f69a20^` where the timeline replaced it, and drawn as the flight deck's
// default view. The fleet as a map: each Job and Session a star, its colour and glow from how it stands, the one
// that needs the owner the brightest. Lines join what is connected; repositories are faint clusters. Nothing moves,
// and nothing pulses. Mock only. proto/cockpit-three-directions.

import { useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { Crosshair, Minus, Plus } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import type { Item } from "../../Dashboard";
import type { Field, Link } from "../../cockpit/map-layout";
import type { DeckSky } from "./deck-sky";

const LINE_SAID: Record<Link["kind"], string> = { dispatched: "Dispatched", waits: "Waits on", works: "Holds", line: "On the merge line" };

/** How bright a star burns: what needs the owner most, what is over least. */
function burn(item: Item): { r: number; halo: number; alpha: number } {
  if (item.hue === "ask" || item.hue === "issue") return { r: 9, halo: 30, alpha: 1 };
  if (item.hue === "running") return { r: 6, halo: 15, alpha: 0.95 };
  if (item.hue === "queued") return { r: 4.5, halo: 0, alpha: 0.75 };
  return { r: 5, halo: 10, alpha: 0.65 };
}

/** Four corner brackets round a point: the selected node's reticle. */
function bracketsOf(x: number, y: number, h: number): string {
  const leg = 7;
  return [
    [-1, -1],
    [1, -1],
    [1, 1],
    [-1, 1],
  ]
    .map(([sx, sy]) => `M${x + sx! * h},${y + sy! * (h - leg)} V${y + sy! * h} H${x + sx! * (h - leg)}`)
    .join(" ");
}

const short = (title: string, room: number) => (title.length > room ? `${title.slice(0, room - 1)}…` : title);

export function StarMap({
  sky,
  field,
  onField,
  selected,
  onSelect,
  onKey,
}: {
  sky: DeckSky;
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
  // Zoom is a window onto the field: 1 is all of it, and the window is kept inside it. Locate moves it to the selection.
  const [zoom, setZoom] = useState(1);
  const [aim, setAim] = useState<{ x: number; y: number }>();
  const seen = { width: field.width / zoom, height: field.height / zoom };
  const centre = {
    x: Math.min(field.width - seen.width / 2, Math.max(seen.width / 2, aim?.x ?? field.width / 2)),
    y: Math.min(field.height - seen.height / 2, Math.max(seen.height / 2, aim?.y ?? field.height / 2)),
  };
  const [tip, setTip] = useState<{ said: string; title: string; left: number; top: number }>();
  const place = new Map<string, { x: number; y: number }>([...sky.stars.map((s) => [s.key, s] as const), ...sky.stars.map((s) => [s.item.job?.id ?? s.key, s] as const), ...sky.landmarks.map((m) => [m.key, m] as const)]);

  const locate = sky.stars.find((one) => one.key === selected);
  return (
    <div ref={host} className="armada-map" data-selected={selected}>
      <svg className="armada-map__field" viewBox={`${centre.x - seen.width / 2} ${centre.y - seen.height / 2} ${seen.width} ${seen.height}`} role="listbox" aria-label="Map" tabIndex={0} onKeyDown={onKey}>
        {sky.dust.map((one, index) => (
          <circle key={index} className="armada-map__dust" cx={one.x} cy={one.y} r={one.r} />
        ))}
        {sky.regions.map((one) => (
          <g key={one.id} className="armada-map__cluster" aria-hidden="true">
            <rect x={one.x} y={one.y} width={one.w} height={one.h} rx={10} />
            <text x={one.x + 12} y={one.y + 17}>
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
              {on ? (
                <g className="armada-star__reticle" aria-hidden="true">
                  <circle className="armada-star__ring" cx={star.x} cy={star.y} r={lit.r + 7} />
                  <path className="armada-star__brackets" d={bracketsOf(star.x, star.y, lit.r + 16)} />
                </g>
              ) : null}
              <text className="armada-star__label" x={star.x} y={star.y + lit.r + 16} textAnchor="middle">
                {short(item.title, sky.chars[star.key] ?? 18)}
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
      <div className="armada-map__zoom">
        <Tooltip label="Zoom in" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom in" disabled={zoom >= 3} onClick={() => setZoom((was) => Math.min(3, was + 0.5))}>
            <Plus size={16} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Zoom out" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom out" disabled={zoom <= 1} onClick={() => setZoom((was) => Math.max(1, was - 0.5))}>
            <Minus size={16} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Locate the selection" asChild>
          <Button variant="ghost" iconOnly aria-label="Locate the selection" disabled={locate === undefined} onClick={() => setAim(locate)}>
            <Crosshair size={16} aria-hidden />
          </Button>
        </Tooltip>
      </div>
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

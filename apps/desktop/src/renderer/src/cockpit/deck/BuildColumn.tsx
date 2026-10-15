// The right column while a Session is open in the deck's centre: its ring drawn tall, with the road to main as the arcs round it, in
// the place of the bays. It draws what the field draws from the same reading (`build.ts`, `ring.tsx`). No words but the tooltips;
// only the piece being built moves, and the subagents running sit inside the ring, by the pieces they wrote. #2301.

import type { ReactNode } from "react";
import { Tooltip } from "@armada/components";

import { SAID } from "./BuildCard";
import { SessionGlyph } from "./Drone";
import { RingArt } from "./ring";
import { arcSaid, arcWord, arcsOf, askedOf, bandOf, glyphOf, roadOf, seatedWorkers, wordOf, type BuildIn } from "./build";
import { AskUpdate } from "./AskUpdate";
import "./build.css";

/** A part of the ring that says what it is when it is hovered or focused. */
const spoken = (said: string, part: ReactNode): ReactNode => (
  <Tooltip label={said} asChild>
    <g className="bf-col__piece" role="img" aria-label={said}>
      {part}
    </g>
  </Tooltip>
);

function Structure({ build }: { build: BuildIn }) {
  const arcs = arcsOf(build);
  const road = roadOf(build);
  return (
    <svg className="bf-col__art" viewBox="-44 -44 88 88" preserveAspectRatio="xMidYMid meet">
      {build.stalled ? <ellipse className="bf-col__stall" cx={0} cy={0} rx={42} ry={42} /> : null}
      <RingArt
        pieces={build.tasks}
        big={1}
        dim={build.dim}
        band={bandOf(build)}
        wrapBand={(band) => spoken(wordOf(build), band)}
        {...(road === undefined ? {} : { arcs: road })}
        wrap={(k, piece) => {
          const task = build.tasks[k]!;
          const said = `${task.text}, ${(task.said ?? SAID[task.piece]).toLowerCase()}`;
          return spoken(task.by === undefined ? said : `${said}, ${askedOf(task.by)}`, piece);
        }}
        wrapArc={(k, arc) => {
          const one = arcs[k]!;
          return spoken(`${arcWord(one.id)}, ${arcSaid(one, build.now)}`, arc);
        }}
      />
      {seated(build).map(({ label, x, y }) => (
        <Tooltip key={label} label={askedOf(label)} asChild>
          <g className="bf-col__worker" role="img" aria-label={`Subagent, ${askedOf(label)}`} tabIndex={0} transform={`translate(${x.toFixed(1)} ${y.toFixed(1)})`}>
            <g transform="scale(0.2)">
              <SessionGlyph />
            </g>
            <circle className="bf-col__hit" r={4.5} />
          </g>
        </Tooltip>
      ))}
    </svg>
  );
}

/** Where each running subagent sits inside the ring: beside its own stretch, a step inward where the stretches are too close for the drones. One with no stretch is spread about the middle. */
function seated(build: BuildIn): { label: string; x: number; y: number }[] {
  const at = (deg: number, r: number) => ({ x: r * Math.cos((deg * Math.PI) / 180), y: r * Math.sin((deg * Math.PI) / 180) });
  const own = new Map(seatedWorkers(build.tasks, build.workers).map((one) => [one.label, one] as const));
  const loose = build.workers.filter((label) => !own.has(label));
  return build.workers.map((label) => {
    const seat = own.get(label);
    if (seat !== undefined) return { label, ...at(seat.angle, seat.radius) };
    return { label, ...(loose.length === 1 ? { x: 0, y: 0 } : at(-90 + (loose.indexOf(label) * 360) / loose.length, 4)) };
  });
}

export function BuildColumn({ build }: { build: BuildIn }) {
  const PhaseGlyph = glyphOf(build);
  return (
    <section className="deck-panel bf-col" aria-label="Build">
      <header className="deck-band">
        <Tooltip label={wordOf(build)} asChild>
          <span className="bf-col__glyph" role="img" aria-label={wordOf(build)}>
            <PhaseGlyph size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
        <AskUpdate subject={build.key} />
      </header>
      <div className="bf-col__body">
        <Structure build={build} />
      </div>
    </section>
  );
}

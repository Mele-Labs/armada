// The right column while a Session is open in the deck's centre: its construction drawn tall, the structure of the current take at the
// top and the road to main running down beneath it as a column of stops, in the place of the bays. It draws what the field draws from
// the same reading (`progress.ts`, `forms.tsx`). No words but the tooltips; only the piece being built moves. #2301.

import type { ReactNode } from "react";
import { Tooltip } from "@armada/components";

import { SAID } from "./BuildCard";
import { GantryArt, RingArt, TowerArt } from "./forms";
import { PHASE_GLYPH, PHASE_WORD, STOP_GLYPH, STOP_WORD, agoOf, glyphOf, wordOf, type Built, type Take } from "./progress";
import { useBuilds } from "./progress-source";
import "./progress.css";

const ART = { tower: TowerArt, pad: GantryArt, ring: RingArt } as const;
const VIEW: Record<Take, string> = { tower: "-36 -92 72 108", pad: "-36 -92 72 108", ring: "-44 -44 88 88" };

function Structure({ built, take }: { built: Built; take: Take }) {
  const Art = ART[take];
  const wrap = (k: number, piece: ReactNode): ReactNode => {
    const task = built.tasks[k]!;
    const said = `${task.text}, ${SAID[task.piece].toLowerCase()}`;
    return (
      <Tooltip label={said} asChild>
        <g className="bf-col__piece" role="img" aria-label={said}>
          {piece}
        </g>
      </Tooltip>
    );
  };
  const ring = take === "ring";
  return (
    <svg className="bf-col__art" data-take={take} viewBox={VIEW[take]} preserveAspectRatio="xMidYMid meet">
      {built.stalled ? <ellipse className="bf-col__stall" cx={0} cy={ring ? 0 : 4} rx={ring ? 42 : 30} ry={ring ? 42 : 9} /> : null}
      <Art pieces={built.tasks} big={1} dim={built.dim} landed={built.landedAt !== undefined} working={false} fly={{ x: 0, y: 0 }} wrap={wrap} />
    </svg>
  );
}

export function BuildColumn({ subjectKey }: { subjectKey: string }) {
  const { by, take } = useBuilds();
  const built = by.get(subjectKey);
  if (built === undefined) return null;
  const PhaseGlyph = glyphOf(built);
  const Landed = PHASE_GLYPH.landed;
  const legs = [
    ...built.stops.map((stop) => ({ id: stop.id, Glyph: STOP_GLYPH[stop.id], word: STOP_WORD[stop.id], at: stop.at })),
    { id: "landed", Glyph: Landed, word: PHASE_WORD.landed, at: built.landedAt },
  ];
  return (
    <section className="deck-panel bf-col" aria-label="Build">
      <header className="deck-band">
        <Tooltip label={wordOf(built)} asChild>
          <span className="bf-col__glyph" role="img" aria-label={wordOf(built)}>
            <PhaseGlyph size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
      </header>
      <div className="bf-col__body">
        <Structure built={built} take={take} />
        <ol className="bf-col__road" aria-label="Road to main">
          {legs.map((leg) => {
            const said = leg.at === undefined ? `${leg.word}, usually next on this repo` : `${leg.word}, ${agoOf(built.now - leg.at)}`;
            return (
              <li key={leg.id} className="bf-col__leg" data-lit={leg.at !== undefined || undefined}>
                <i className="bf-col__link" />
                <Tooltip label={said} asChild>
                  <span className="bf-col__stop" role="img" aria-label={said}>
                    <leg.Glyph size={14} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
              </li>
            );
          })}
        </ol>
      </div>
    </section>
  );
}

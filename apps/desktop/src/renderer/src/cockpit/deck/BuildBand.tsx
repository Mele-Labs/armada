// A Session's or a Job's construction as a band across the top of its page: the structure of the current take, side on, cut out of the
// field, and the road to main as a line of stops to its right. It draws what the field draws from the same reading (`progress.ts`,
// `forms.tsx`), so the two cannot disagree. Only the piece being built moves. Hover a piece for its task and state, a stop for its name
// and when. #2301.

import type { ReactNode } from "react";
import { Tooltip } from "@armada/components";

import { SAID } from "./BuildCard";
import { GantryArt, RingArt, TowerArt } from "./forms";
import { PHASE_GLYPH, STOP_GLYPH, STOP_WORD, agoOf, glyphOf, wordOf, PHASE_WORD, type Built, type Take } from "./progress";
import { useBuilds } from "./progress-source";
import "./progress.css";

const ART = { tower: TowerArt, pad: GantryArt, ring: RingArt } as const;
const VIEW: Record<Take, string> = { tower: "-36 -92 72 108", pad: "-36 -92 72 108", ring: "-44 -24 88 48" };

function Structure({ built, take }: { built: Built; take: Take }) {
  const Art = ART[take];
  const wrap = (k: number, piece: ReactNode): ReactNode => {
    const task = built.tasks[k]!;
    const said = `${task.text}, ${SAID[task.piece].toLowerCase()}`;
    return (
      <Tooltip label={said} asChild>
        <g className="bf-band__piece" role="img" aria-label={said}>
          {piece}
        </g>
      </Tooltip>
    );
  };
  return (
    <svg className="bf-band__art" data-take={take} viewBox={VIEW[take]} preserveAspectRatio="xMidYMax meet">
      {built.stalled ? <ellipse className="bf-band__stall" cx={0} cy={take === "ring" ? 0 : 4} rx={take === "ring" ? 42 : 30} ry={take === "ring" ? 22 : 9} /> : null}
      <g transform={take === "ring" ? "scale(1 0.5)" : undefined}>
        <Art pieces={built.tasks} big={take === "ring" ? 1.1 : 1} dim={built.dim} landed={built.landedAt !== undefined} working={false} fly={{ x: 0, y: 0 }} wrap={wrap} />
      </g>
    </svg>
  );
}

export function BuildBand({ subjectKey }: { subjectKey: string }) {
  const { by, take } = useBuilds();
  const built = by.get(subjectKey);
  if (built === undefined || built.tasks.length === 0) return null;
  const job = subjectKey.startsWith("job:");
  const faint = job ? "later in its workflow" : "usually next on this repo";
  const PhaseGlyph = glyphOf(built);
  const Landed = PHASE_GLYPH.landed;
  const lit = (at: number | undefined) => at !== undefined;
  return (
    <div className="bf-band" role="group" aria-label="Build">
      <div className="bf-band__phase">
        <Tooltip label={wordOf(built)} asChild>
          <span className="bf-band__glyph" role="img" aria-label={wordOf(built)}>
            <PhaseGlyph size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
        {wordOf(built)}
        {built.stalled ? <span className="bf-band__quiet">{`Quiet ${built.quiet} min`}</span> : null}
      </div>
      <Structure built={built} take={take} />
      <ol className="bf-band__road" aria-label="Road to main">
        {built.stops.map((stop) => {
          const Glyph = STOP_GLYPH[stop.id];
          const said = lit(stop.at) ? `${STOP_WORD[stop.id]}, ${agoOf(built.now - stop.at!)}` : `${STOP_WORD[stop.id]}, ${faint}`;
          return (
            <li key={stop.id} className="bf-band__leg" data-lit={lit(stop.at) || undefined}>
              <i className="bf-band__link" />
              <Tooltip label={said} asChild>
                <span className="bf-band__stop" role="img" aria-label={said}>
                  <Glyph size={14} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
            </li>
          );
        })}
        <li className="bf-band__leg" data-lit={built.landedAt !== undefined || undefined}>
          <i className="bf-band__link" />
          <Tooltip label={built.landedAt === undefined ? `${PHASE_WORD.landed}, ${faint}` : `${PHASE_WORD.landed}, ${agoOf(built.now - built.landedAt)}`} asChild>
            <span className="bf-band__stop" role="img" aria-label={PHASE_WORD.landed}>
              <Landed size={14} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
        </li>
      </ol>
    </div>
  );
}

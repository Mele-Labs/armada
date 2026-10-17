// The ring: a segment for each task, or one unsegmented band where there is no list, and the road to main as a thinner ring of arcs outside it. A piece is ghost (dashed, planned),
// scaffold (the first task not done, edits seen since), hollow (done, no edit seen) or solid. A piece is static once placed. #2301.

import type { ReactNode } from "react";

import type { Arc, Band, Piece } from "./build";

export type RingProps = {
  pieces: readonly { piece: Piece; by?: string }[];
  big: number;
  /** How far the unfinished part has dimmed, 0 to 1. */
  dim: number;
  /** The road, one arc for each stop and the landing last. Absent where the ring has none. */
  arcs?: readonly Arc[];
  /** Draws a piece inside something that speaks for it when it is hovered. */
  wrap?: (k: number, piece: ReactNode) => ReactNode;
  /** Speaks for an arc, as `wrap` does for a piece. */
  wrapArc?: (k: number, arc: ReactNode) => ReactNode;
  /** The one band that stands for the ring where there are no pieces. Ignored where there are some. */
  band?: Band | undefined;
  /** Speaks for the band. */
  wrapBand?: (band: ReactNode) => ReactNode;
};
const plain = (_k: number, part: ReactNode) => part;
const whole = (part: ReactNode) => part;
const open = (piece: Piece) => piece === "ghost" || piece === "scaffold";

/** One segment of a ring: an annular sector from angle `from` to `to`, in degrees, between radii r0 and r1. */
const sector = (r0: number, r1: number, from: number, to: number) => {
  const p = (r: number, deg: number) => `${(r * Math.cos((deg * Math.PI) / 180)).toFixed(2)},${(r * Math.sin((deg * Math.PI) / 180)).toFixed(2)}`;
  return `M${p(r1, from)} A${r1},${r1} 0 0 1 ${p(r1, to)} L${p(r0, to)} A${r0},${r0} 0 0 0 ${p(r0, from)} Z`;
};

/** The extra gap, in degrees, at each end of a subagent's stretch. */
const WIDER = 1.5;
/** How many pieces in a row, from `k`, were written by the same subagent. */
const runOf = (pieces: readonly { by?: string }[], k: number) => {
  let length = 1;
  while (pieces[k + length]?.by === pieces[k]!.by) length += 1;
  return { length };
};
/** A thin arc at radius r between two angles in degrees. */
const strip = (r: number, from: number, to: number) => {
  const p = (deg: number) => `${(r * Math.cos((deg * Math.PI) / 180)).toFixed(2)},${(r * Math.sin((deg * Math.PI) / 180)).toFixed(2)}`;
  return `M${p(from)} A${r},${r} 0 ${to - from > 180 ? 1 : 0} 1 ${p(to)}`;
};

/** A full annulus between radii r0 and r1, as two circles filled even-odd. */
const annulus = (r0: number, r1: number) => `M${r1},0 A${r1},${r1} 0 1 1 ${-r1},0 A${r1},${r1} 0 1 1 ${r1},0 Z M${r0},0 A${r0},${r0} 0 1 0 ${-r0},0 A${r0},${r0} 0 1 0 ${r0},0 Z`;

/** The task ring in the plane's own units, with the road as a ring of its own outside it: 4.5 thick, a gap of 4. */
export function RingArt({ pieces, big, dim, arcs = [], wrap = plain, wrapArc = plain, band, wrapBand = whole }: RingProps) {
  const n = Math.max(pieces.length, 1);
  const r1 = 34 * big;
  const r0 = r1 - 7 * big;
  const [a0, a1] = [r1 + 4 * big, r1 + 8.5 * big];
  const m = Math.max(arcs.length, 1);
  // A subagent's stretch is set off by a wider gap at each end and a thin arc inside it, so it reads as one.
  const edge = (k: number) => pieces[k]?.by !== pieces[(k + n - 1) % n]?.by && pieces.length > 1;
  const groups = pieces.flatMap((one, k) => (one.by === undefined || (k > 0 && pieces[k - 1]!.by === one.by) ? [] : [{ k, ...runOf(pieces, k) }]));
  return (
    <g className="bf-form" style={{ ["--dim" as string]: dim }} data-form="ring">
      {pieces.length > 0 || band === undefined
        ? null
        : wrapBand(
            <g className="bf-band" data-state={band.state} data-open={band.state !== "solid" || undefined} data-moving={band.moving || undefined}>
              <path className="bf-band__seg" d={annulus(r0, r1)} fillRule="evenodd" />
              {band.state === "scaffold" ? <circle className="bf-band__tex" cx={0} cy={0} r={(r0 + r1) / 2} strokeWidth={r1 - r0} /> : null}
            </g>,
          )}
      {pieces.map((one, k) => {
        const from = -90 + (k * 360) / n + 2.5 + (edge(k) ? WIDER : 0);
        const to = -90 + ((k + 1) * 360) / n - 2.5 - (edge((k + 1) % n) ? WIDER : 0);
        const mid = ((from + to) / 2) * (Math.PI / 180);
        return (
          <g key={k}>
            {wrap(
              k,
              <g className="bf-pc" data-state={one.piece} data-open={open(one.piece) || undefined} data-by={one.by !== undefined || undefined}>
                <path className="bf-pc__seg" d={sector(r0, r1, from, to)} />
                {one.by === undefined ? null : <circle className="bf-pc__by" cx={(((r0 + r1) / 2) * Math.cos(mid)).toFixed(1)} cy={(((r0 + r1) / 2) * Math.sin(mid)).toFixed(1)} r={1.7 * big} />}
                {one.piece === "scaffold" ? <path className="bf-pc__brace" d={`M${((r1 + 2) * Math.cos(mid)).toFixed(1)},${((r1 + 2) * Math.sin(mid)).toFixed(1)} L${((r1 + 9) * Math.cos(mid)).toFixed(1)},${((r1 + 9) * Math.sin(mid)).toFixed(1)}`} /> : null}
              </g>,
            )}
          </g>
        );
      })}
      {groups.map((group) => (
        <path key={group.k} className="bf-grp" d={strip(r0 - 2 * big, -90 + (group.k * 360) / n + 2.5 + WIDER, -90 + ((group.k + group.length) * 360) / n - 2.5 - WIDER)} />
      ))}
      {arcs.map((arc, k) => (
        <g key={arc.id}>
          {wrapArc(
            k,
            <g className="bf-arc" data-lit={arc.at !== undefined || undefined}>
              <path className="bf-arc__seg" d={sector(a0, a1, -90 + (k * 360) / m + 3, -90 + ((k + 1) * 360) / m - 3)} />
            </g>,
          )}
        </g>
      ))}
    </g>
  );
}

/** A ring lying on the ground round the drone's shadow. It is on the plane, so the tilt makes it an ellipse. */
export function Ring(props: RingProps) {
  const edge = 34 * props.big + 12;
  return (
    <svg className="bf-ring" style={{ left: -edge, top: -edge }} width={edge * 2} height={edge * 2} viewBox={`${-edge} ${-edge} ${edge * 2} ${edge * 2}`}>
      <RingArt {...props} />
    </svg>
  );
}

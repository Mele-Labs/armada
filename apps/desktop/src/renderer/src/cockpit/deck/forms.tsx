// The three forms a structure takes on its plot, one piece for each task: a tower rising floor by floor, a launch gantry with a craft
// assembled on it, a segmented ring on the ground. A piece is ghost (dashed, planned), scaffold (the next ghost, edits seen), hollow
// (ticked, no edit seen) or solid. A piece is static once placed; only the crane's jib swings and a craft lifts off, once. #2301.

import type { ReactNode } from "react";

import { Sprite } from "./bf-parts";
import type { Arc, Piece, RoadAt } from "./progress";

type Pieces = readonly { piece: Piece }[];
export type FormProps = {
  pieces: Pieces;
  big: number;
  /** How far the unfinished part has dimmed, 0 to 1. */
  dim: number;
  /** It has landed: the top is finished and, for a craft, it flies. */
  landed: boolean;
  /** Its crane swings: the drone is writing. */
  working: boolean;
  /** Where the craft goes, in plane units from where it stands. */
  fly: { x: number; y: number };
  /** Draws a piece inside something that speaks for it, as the band does to say what each is when it is hovered. */
  wrap?: (k: number, piece: ReactNode) => ReactNode;
  /** The ring's road, one arc for each stop and the landing last, and which side of the task ring it lies on. */
  arcs?: readonly Arc[];
  road?: RoadAt;
  /** Speaks for an arc, as `wrap` does for a piece. */
  wrapArc?: (k: number, arc: ReactNode) => ReactNode;
  /** Drawn in the middle of the ring, inside the road where the road is inside. */
  centre?: ReactNode;
};
const plain = (_k: number, piece: ReactNode) => piece;

const A = 21;
const D = 10.5;
const H = 8;
/** The faces of a box of half-width a and half-depth d, height h, standing on base line b. */
const faces = (a: number, d: number, h: number, b: number) => ({
  top: `${-a},${b - h} 0,${b - h - d} ${a},${b - h} 0,${b - h + d}`,
  left: `${-a},${b - h} 0,${b - h + d} 0,${b + d} ${-a},${b}`,
  right: `${a},${b - h} 0,${b - h + d} 0,${b + d} ${a},${b}`,
  line: `M${-a},${b - h} L0,${b - h + d} L${a},${b - h} V${b} L0,${b + d} L${-a},${b} Z M0,${b - h + d} V${b + d}`,
});

/** The braces round a floor: four posts and a cross on each face, in the colour of the laser that is welding it. */
const braces = (a: number, d: number, h: number, b: number) =>
  `M${-a},${b} V${b - h - 3} M${a},${b} V${b - h - 3} M0,${b + d} V${b - h + d - 3} M${-a},${b - h} L0,${b + d} M${-a},${b} L0,${b - h + d} M${a},${b - h} L0,${b + d} M${a},${b} L0,${b - h + d}`;

const open = (piece: Piece) => piece === "ghost" || piece === "scaffold";

function Floor({ piece, k }: { piece: Piece; k: number }) {
  const b = -k * H;
  const f = faces(A, D, H, b);
  return (
    <g className="bf-pc" data-state={piece} data-open={open(piece) || undefined}>
      {piece === "solid" ? (
        <>
          <polygon className="bf-pc__face" data-face="left" points={f.left} />
          <polygon className="bf-pc__face" data-face="right" points={f.right} />
          <polygon className="bf-pc__face" data-face="top" points={f.top} />
        </>
      ) : (
        <path className="bf-pc__line" d={f.line} />
      )}
      {piece === "scaffold" ? <path className="bf-pc__brace" d={braces(A, D, H, b)} /> : null}
    </g>
  );
}

/** A building: a floor for each task on a diamond footprint, a crane on the highest floor that is placed, a pennant when it lands. */
export function TowerArt({ pieces, big, dim, landed, working, wrap = plain }: FormProps) {
  const s = big * 0.9;
  const placed = pieces.reduce((n, one, i) => (one.piece === "ghost" ? n : i + 1), 0);
  return (
    <g className="bf-form" style={{ ["--dim" as string]: dim }} data-form="tower" transform={`scale(${s})`}>
        <polygon className="bf-pc__pad" points={`${-A - 8},0 0,${-D - 4} ${A + 8},0 0,${D + 4}`} transform={`translate(0 ${D * 0.2})`} />
        {pieces.map((one, k) => <g key={k}>{wrap(k, <Floor piece={one.piece} k={k} />)}</g>)}
        {pieces.length === 0 || placed === 0 ? null : (
          <g className="bf-crane" transform={`translate(${A * 0.4} ${-placed * H - H * 0.4})`}>
            <path className="bf-crane__mast" d="M0,0 V-18" />
            <g className="bf-crane__jib" data-swing={working || undefined}>
              <path className="bf-crane__arm" d="M-6,-18 H24 M0,-18 L-6,-14 M24,-18 V-10" />
            </g>
          </g>
        )}
        {landed && pieces.length > 0 ? (
          <g className="bf-pc__flag" transform={`translate(0 ${-pieces.length * H - D * 0.4})`}>
            <path className="bf-pc__pole" d="M0,0 V-14" />
            <path className="bf-pc__pennant" d="M0,-14 L10,-11 L0,-8 Z" />
          </g>
        ) : null}
    </g>
  );
}

export const Tower = (props: FormProps) => (
  <Sprite x={-50} y={-150} w={100} h={176}>
    <TowerArt {...props} />
  </Sprite>
);

const W = 12;
const S = 9;
function Section({ piece, k, nose }: { piece: Piece; k: number; nose: boolean }) {
  const b = -k * S;
  return (
    <g className="bf-pc" data-state={piece} data-open={open(piece) || undefined}>
      {nose ? (
        <path className="bf-pc__body" d={`M${-W / 2},${b - S} C${-W / 2},${b - S - 8} -2,${b - S - 13} 0,${b - S - 13} C2,${b - S - 13} ${W / 2},${b - S - 8} ${W / 2},${b - S} Z`} />
      ) : (
        <>
          <rect className="bf-pc__body" x={-W / 2} y={b - S} width={W} height={S} />
          <ellipse className="bf-pc__cap" cy={b - S} rx={W / 2} ry={2.4} />
        </>
      )}
      {piece === "scaffold" ? <path className="bf-pc__brace" d={`M${-W / 2 - 3},${b} V${b - S - 2} M${W / 2 + 3},${b} V${b - S - 2} M${-W / 2 - 3},${b - S} L${W / 2 + 3},${b}`} /> : null}
    </g>
  );
}

/** A launch gantry with a craft on it: a section for each task, the nose last, the service arm at the section being fitted. */
export function GantryArt({ pieces, big, dim, landed, working, fly, wrap = plain }: FormProps) {
  const s = big * 0.9;
  const tall = Math.max(pieces.length, 1) * S + 16;
  const fitting = Math.max(0, pieces.findIndex((one) => one.piece !== "solid" && one.piece !== "hollow"));
  const rail = (x: number) => `M${x - 2},0 V${-tall} M${x + 2},0 V${-tall} ${Array.from({ length: Math.floor(tall / 8) }, (_, j) => `M${x - 2},${-j * 8} L${x + 2},${-j * 8 - 8}`).join(" ")}`;
  return (
    <g className="bf-form" style={{ ["--dim" as string]: dim }} data-form="pad" transform={`scale(${s})`}>
        <polygon className="bf-pc__pad" points={`-30,0 0,-15 30,0 0,15`} />
        <path className="bf-gantry" d={`${rail(-17)} ${rail(17)}`} />
        {pieces.length === 0 ? null : <path className="bf-gantry__arm" d={`M17,${-fitting * S - 4} H8`} />}
        <g className="bf-craft" data-flown={landed || undefined} style={{ ["--fx" as string]: `${fly.x.toFixed(0)}px`, ["--fy" as string]: `${fly.y.toFixed(0)}px` }}>
          {pieces.map((one, k) => <g key={k}>{wrap(k, <Section piece={one.piece} k={k} nose={k === pieces.length - 1} />)}</g>)}
        </g>
        <path className="bf-gantry__hook bf-crane__jib" data-swing={working || undefined} d={`M17,${-tall} H5`} />
    </g>
  );
}

export const Gantry = (props: FormProps) => (
  <Sprite x={-50} y={-150} w={100} h={176}>
    <GantryArt {...props} />
  </Sprite>
);

/** One segment of a ring: an annular sector from angle `from` to `to`, in degrees, between radii r0 and r1. */
const sector = (r0: number, r1: number, from: number, to: number) => {
  const p = (r: number, deg: number) => `${(r * Math.cos((deg * Math.PI) / 180)).toFixed(2)},${(r * Math.sin((deg * Math.PI) / 180)).toFixed(2)}`;
  return `M${p(r1, from)} A${r1},${r1} 0 0 1 ${p(r1, to)} L${p(r0, to)} A${r0},${r0} 0 0 0 ${p(r0, from)} Z`;
};

/** The segments of a ring, a segment for each task, in the plane's own units, and the road as a thinner ring of arcs outside or inside it. */
export function RingArt({ pieces, big, dim, arcs = [], road = "outer", wrap = plain, wrapArc = plain, centre }: FormProps) {
  const n = Math.max(pieces.length, 1);
  const r1 = 34 * big;
  const r0 = r1 - 7 * big;
  // The road is a ring of its own, 4.5 thick, a gap of 4 from the task ring.
  const [a0, a1] = road === "outer" ? [r1 + 4 * big, r1 + 8.5 * big] : [r0 - 8.5 * big, r0 - 4 * big];
  const m = Math.max(arcs.length, 1);
  return (
    <g className="bf-form" style={{ ["--dim" as string]: dim }} data-form="ring">
      {pieces.map((one, k) => {
        const from = -90 + (k * 360) / n + 2.5;
        const to = -90 + ((k + 1) * 360) / n - 2.5;
        const mid = ((from + to) / 2) * (Math.PI / 180);
        return (
          <g key={k}>
            {wrap(
              k,
              <g className="bf-pc" data-state={one.piece} data-open={open(one.piece) || undefined}>
                <path className="bf-pc__seg" d={sector(r0, r1, from, to)} />
                {one.piece === "scaffold" ? <path className="bf-pc__brace" d={`M${((r1 + 2) * Math.cos(mid)).toFixed(1)},${((r1 + 2) * Math.sin(mid)).toFixed(1)} L${((r1 + 9) * Math.cos(mid)).toFixed(1)},${((r1 + 9) * Math.sin(mid)).toFixed(1)}`} /> : null}
              </g>,
            )}
          </g>
        );
      })}
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
      {centre}
    </g>
  );
}

/** A ring lying on the ground round the drone's shadow. It is on the plane, so the tilt makes it an ellipse. */
export function Ring(props: FormProps) {
  const edge = 34 * props.big + 12;
  return (
    <svg className="bf-ring" style={{ left: -edge, top: -edge }} width={edge * 2} height={edge * 2} viewBox={`${-edge} ${-edge} ${edge * 2} ${edge * 2}`} aria-hidden="true">
      <RingArt {...props} />
    </svg>
  );
}

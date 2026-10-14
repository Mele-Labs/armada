// The flight deck's map layout. The Cockpit's `skyOf` clusters every repository in a ring and packs stars by a
// spiral, which is tight in a pane this size. Here the same stars, links and landmarks are laid out across the whole
// pane: each repository gets a region of it, sized by how many stars it holds, and its stars sit on a grid of cells
// no smaller than a label, so no two labels meet. Mock only; `map-layout.ts` is untouched.

import type { Session } from "@armada/screens/src/draft/sessions";
import type { MergeLineView } from "@armada/screens";

import type { Item } from "../../Dashboard";
import { skyOf, type Field, type Landmark, type Sky, type Star } from "../../cockpit/map-layout";

export type Region = { id: string; name: string; x: number; y: number; w: number; h: number };
/** `chars` is how many letters of a title its star's label may show in the room that star has. */
export type DeckSky = Sky & { regions: Region[]; chars: Record<string, number> };

type Rect = { x: number; y: number; w: number; h: number };

const MARGIN = 12;
const GAP = 12;
const HEAD = 26;
const FOOT = 44;
const SIDE = 8;
const LETTER = 6.6;
const MAX_CHARS = 26;
const MIN_CHARS = 9;
const BOX_H = 56;

/** Split a rect among weighted parts, along its longer side, so each part's area follows its weight. */
function slice(parts: readonly { id: string; weight: number }[], rect: Rect, into: Map<string, Rect>): void {
  if (parts.length === 1) {
    into.set(parts[0]!.id, rect);
    return;
  }
  const sorted = [...parts].sort((a, b) => b.weight - a.weight || a.id.localeCompare(b.id));
  const total = sorted.reduce((sum, one) => sum + one.weight, 0);
  let take = 0;
  let sum = 0;
  while (take < sorted.length - 1 && (take === 0 || sum + sorted[take]!.weight / 2 <= total / 2)) sum += sorted[take++]!.weight;
  const first = sorted.slice(0, take);
  const rest = sorted.slice(take);
  const share = sum / total;
  if (rect.w >= rect.h) {
    const left = (rect.w - GAP) * share;
    slice(first, { x: rect.x, y: rect.y, w: left, h: rect.h }, into);
    slice(rest, { x: rect.x + left + GAP, y: rect.y, w: rect.w - left - GAP, h: rect.h }, into);
  } else {
    const top = (rect.h - GAP) * share;
    slice(first, { x: rect.x, y: rect.y, w: rect.w, h: top }, into);
    slice(rest, { x: rect.x, y: rect.y + top + GAP, w: rect.w, h: rect.h - top - GAP }, into);
  }
}

/** A fixed number in [-1, 1] from a key, so the same star is nudged the same way every time. */
function nudge(key: string, salt: number): number {
  let hash = 2166136261 ^ salt;
  for (let i = 0; i < key.length; i += 1) hash = Math.imul(hash ^ key.charCodeAt(i), 16777619);
  return ((hash >>> 0) % 2001) / 1000 - 1;
}

/** The grid that gives each of `count` labels the most room in `area`, shortening labels only when it must. */
function gridOf(count: number, area: Rect): { cols: number; rows: number; chars: number } {
  let best = { cols: 1, rows: Math.max(1, count), chars: MIN_CHARS, room: -Infinity };
  for (let chars = MAX_CHARS; chars >= MIN_CHARS; chars -= 1) {
    const boxW = chars * LETTER + 14;
    const top = { cols: 1, rows: Math.max(1, count), room: -Infinity };
    for (let cols = 1; cols <= Math.max(1, count); cols += 1) {
      const rows = Math.ceil(count / cols);
      const room = Math.min(area.w / cols / boxW, area.h / rows / BOX_H);
      if (room > top.room + 1e-6) Object.assign(top, { cols, rows, room });
    }
    if (top.room > best.room || best.room === -Infinity) best = { ...top, chars };
    if (top.room >= 1) return { cols: top.cols, rows: top.rows, chars };
  }
  return { cols: best.cols, rows: best.rows, chars: best.chars };
}

export function deckSkyOf(
  items: readonly Item[],
  sessions: readonly Session[],
  views: readonly MergeLineView[],
  manifestOf: (root: string) => string | undefined,
  nameOf: (manifest: string) => string,
  field: Field,
): DeckSky {
  const base = skyOf(items, sessions, views, manifestOf, nameOf, field);
  const withLine = new Set(base.landmarks.map((one) => one.cluster));
  const weights = base.clusters.map((one) => ({ id: one.id, weight: base.stars.filter((star) => star.cluster === one.id).length + 1.5 }));
  const rects = new Map<string, Rect>();
  if (weights.length > 0) slice(weights, { x: MARGIN, y: MARGIN, w: Math.max(80, field.width - MARGIN * 2), h: Math.max(80, field.height - MARGIN * 2) }, rects);

  const jobOf = new Map(base.links.filter((one) => one.kind === "works").map((one) => [one.from, one.to] as const));
  const stars: Star[] = [];
  const chars: Record<string, number> = {};
  const regions: Region[] = [];
  const landmarks: Landmark[] = [];

  for (const cluster of base.clusters) {
    const rect = rects.get(cluster.id)!;
    regions.push({ id: cluster.id, name: cluster.name, x: rect.x, y: rect.y, w: rect.w, h: rect.h });
    const own = base.stars.filter((star) => star.cluster === cluster.id);
    // A Job, then the Sessions that hold it, so a line between them is short.
    const jobs = own.filter((star) => star.item.session !== true);
    const order = [...jobs.flatMap((job) => [job, ...own.filter((star) => jobOf.get(star.key) === (job.item.job?.id ?? job.key))]), ...own.filter((star) => star.item.session === true && !jobs.some((job) => jobOf.get(star.key) === (job.item.job?.id ?? job.key)))];

    const top = cluster.name === "" ? 0 : HEAD;
    const foot = withLine.has(cluster.id) ? FOOT : 0;
    const area: Rect = { x: rect.x + SIDE, y: rect.y + top, w: rect.w - SIDE * 2, h: Math.max(BOX_H, rect.h - top - foot) };
    const { cols, rows, chars: fit } = gridOf(order.length, area);
    const cellW = area.w / cols;
    const cellH = area.h / rows;
    const boxW = fit * LETTER + 14;
    order.forEach((star, at) => {
      const row = Math.floor(at / cols);
      const inRow = row === rows - 1 ? order.length - row * cols : cols;
      const col = at - row * cols + (cols - inRow) / 2;
      const slackX = Math.max(0, cellW - boxW) / 2 * 0.7;
      const slackY = Math.max(0, cellH - BOX_H) / 2 * 0.7;
      stars.push({ ...star, x: area.x + (col + 0.5) * cellW + nudge(star.key, 1) * slackX, y: area.y + (row + 0.5) * cellH + nudge(star.key, 2) * slackY - 6 });
      chars[star.key] = fit;
    });
  }

  for (const one of base.landmarks) {
    const rect = rects.get(one.cluster)!;
    const x = rect.x + rect.w - 30;
    const y = rect.y + rect.h - 20;
    landmarks.push({ ...one, x, y, branches: one.branches.map((branch, at) => ({ ...branch, x: x - 26 * (at + 1), y })) });
  }

  const clusters = base.clusters.map((one) => {
    const rect = rects.get(one.id)!;
    return { ...one, x: rect.x + rect.w / 2, y: rect.y + rect.h / 2, r: Math.min(rect.w, rect.h) / 2 };
  });
  return { ...base, stars, landmarks, clusters, regions, chars };
}

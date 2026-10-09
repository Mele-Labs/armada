// The map's geometry and its lines, both pure and deterministic: the same Jobs and Sessions are the
// same stars in the same places on every render, so nothing settles and nothing jitters. A star is
// an item; items cluster by repository, the way a constellation does, and each cluster is placed on
// an ellipse in a fixed order. Lines are only what the data already says: a Job's parent, what it
// waits on, a Session's Job, and the branches on the merge line. Mock only where it says so.

import type { Session } from "@armada/screens/src/draft/sessions";
import type { MergeLineView } from "@armada/screens";

import type { Item } from "../Dashboard";

/** The field's pixels: the map is laid out in the room it has, so its words stay the size they are. */
export type Field = { width: number; height: number };

/** The things a line can say. `works` (a Session and a Job it holds) and `line` are Sessions' and the merge line's. */
export type LinkKind = "dispatched" | "waits" | "works" | "line";

export type Star = {
  /** The item's key, which is what a selection is. */
  key: string;
  item: Item;
  x: number;
  y: number;
  /** The repository's manifest id, or `~` for a Session no Job places. */
  cluster: string;
};

/** The merge line as a place: `main`, and the branches queued for it that no Job here owns. */
export type Landmark = { key: string; label: string; x: number; y: number; red: boolean; cluster: string; branches: readonly { key: string; label: string; x: number; y: number }[] };

export type Link = { from: string; to: string; kind: LinkKind };

export type Cluster = { id: string; name: string; x: number; y: number; r: number };

export type Sky = { stars: Star[]; landmarks: Landmark[]; links: Link[]; clusters: Cluster[]; dust: readonly { x: number; y: number; r: number }[] };

/** What an item is called in links: a Job by its id, anything else by its key. */
const refOf = (item: Item) => item.job?.id ?? item.key;

/** The nth of `count` places round the field's ellipse, starting at the top. */
function centreOf(index: number, count: number, field: Field, radii: readonly number[]): { x: number; y: number } {
  if (count === 1) return { x: field.width / 2, y: field.height / 2 };
  const turn = -Math.PI / 2 + (index / count) * 2 * Math.PI;
  // The ellipse is as wide as the biggest cluster leaves room for, so none runs off the field.
  const biggest = Math.max(...radii);
  const across = Math.max(0, field.width / 2 - biggest - 24);
  const down = Math.max(0, field.height / 2 - biggest - 34);
  return { x: field.width / 2 + Math.cos(turn) * across, y: field.height / 2 + Math.sin(turn) * down };
}

/** A fixed scatter of faint points, from a fixed seed, as shares of the field. */
function dustOf(): readonly { x: number; y: number; r: number }[] {
  let seed = 7;
  const next = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
  return Array.from({ length: 70 }, () => ({ x: next(), y: next(), r: 0.5 + next() * 0.9 }));
}

const DUST = dustOf();

export function skyOf(
  items: readonly Item[],
  sessions: readonly Session[],
  views: readonly MergeLineView[],
  manifestOf: (root: string) => string | undefined,
  nameOf: (manifest: string) => string,
  field: Field,
): Sky {
  const byRef = new Map(items.map((item) => [refOf(item), item]));

  // A Session sits with the Job it dispatched; one with none is on its own.
  const jobOfSession = new Map<string, string>();
  for (const session of sessions) {
    const held = session.attachments.find((one) => one.kind === "job" && one.looking !== true && byRef.has(one.id));
    if (held !== undefined && held.kind === "job") jobOfSession.set(`session:${session.id}`, held.id);
  }
  const clusterOf = (item: Item): string => {
    if (item.job !== undefined) return item.job.owner_manifest_id;
    const job = byRef.get(jobOfSession.get(item.key) ?? "")?.job;
    return job?.owner_manifest_id ?? "~";
  };

  const byCluster = (id: string) => items.filter((item) => clusterOf(item) === id).sort((a, b) => a.key.localeCompare(b.key));
  const ids = [...new Set(items.map(clusterOf))].sort();
  const counts = ids.map((id) => byCluster(id).length);

  // How far apart stars sit: as wide as the room allows, and narrower until every cluster fits in it.
  const radiusOf = (count: number, unit: number) => unit * Math.sqrt(Math.max(0, count - 1)) + unit + 8;
  const fits = (unit: number) => {
    const radii = counts.map((count) => radiusOf(count, unit));
    const rest = Math.max(...radii, 0);
    if (rest * 2 + 70 > Math.min(field.width, field.height) + (ids.length === 1 ? 0 : 0)) return false;
    return ids.every((_, i) => ids.every((__, j) => j <= i || Math.hypot(centreOf(i, ids.length, field, radii).x - centreOf(j, ids.length, field, radii).x, centreOf(i, ids.length, field, radii).y - centreOf(j, ids.length, field, radii).y) >= radii[i]! + radii[j]! + 8));
  };
  let unit = Math.min(60, Math.max(40, Math.min(field.width, field.height) / 10));
  while (unit > 26 && !fits(unit)) unit -= 2;
  const radii = counts.map((count) => radiusOf(count, unit));

  const stars: Star[] = [];
  const clusters: Cluster[] = [];
  ids.forEach((id, index) => {
    const centre = centreOf(index, ids.length, field, radii);
    byCluster(id).forEach((item, at) => {
      // A phyllotaxis spiral: even, and the same every time.
      const angle = at * 2.399963;
      const reach = unit * Math.sqrt(at);
      stars.push({ key: item.key, item, x: centre.x + Math.cos(angle) * reach, y: centre.y + Math.sin(angle) * reach, cluster: id });
    });
    clusters.push({ id, name: id === "~" ? "" : nameOf(id), x: centre.x, y: centre.y, r: radii[index]! });
  });

  const links: Link[] = [];
  const has = (ref: string) => byRef.has(ref);
  for (const item of items) {
    const job = item.job;
    if (job === undefined) continue;
    if (job.dispatched_by !== undefined && has(job.dispatched_by)) links.push({ from: job.dispatched_by, to: job.id, kind: "dispatched" });
    for (const waits of job.waits_on ?? []) if (has(waits)) links.push({ from: job.id, to: waits, kind: "waits" });
  }
  for (const [session, job] of jobOfSession) links.push({ from: session, to: job, kind: "works" });

  // The merge line: `main` stands beside its repository's cluster, the branches queued on it round it.
  const landmarks: Landmark[] = [];
  for (const view of views) {
    const manifest = manifestOf(view.root);
    const cluster = clusters.find((one) => one.id === manifest);
    if (cluster === undefined || view.line.length === 0) continue;
    // `main` stands where the field is emptiest round its repository: the twelfth of the circle that
    // keeps furthest from every star, ring and edge, the first such by angle when two tie.
    const spot = (turn: number) => ({
      x: Math.min(field.width - 70, Math.max(70, cluster.x + Math.cos(turn) * (cluster.r + 56))),
      y: Math.min(field.height - 60, Math.max(60, cluster.y + Math.sin(turn) * (cluster.r + 56))),
    });
    const room = (at: { x: number; y: number }) =>
      Math.min(
        ...stars.map((star) => Math.hypot(star.x - at.x, star.y - at.y)),
        ...landmarks.map((one) => Math.hypot(one.x - at.x, one.y - at.y)),
        ...clusters.filter((one) => one !== cluster).map((one) => Math.hypot(one.x - at.x, one.y - at.y) - one.r),
      );
    let away = 0;
    let most = -Infinity;
    for (let k = 0; k < 12; k += 1) {
      const turn = (k / 12) * 2 * Math.PI;
      const found = room(spot(turn));
      if (found > most + 0.5) {
        most = found;
        away = turn;
      }
    }
    const { x, y } = spot(away);
    const branches = view.line.map((entry, at) => {
      const angle = away + Math.PI / 2 + (at - (view.line.length - 1) / 2) * 0.55;
      const owner = stars.find((star) => star.item.job?.branch === entry.branch);
      return { key: entry.branch, label: entry.branch.split("/").pop() ?? entry.branch, x: x + Math.cos(angle) * 54, y: y + Math.sin(angle) * 54, owner: owner?.key };
    });
    landmarks.push({ key: `main:${view.root}`, label: "main", x, y, red: view.hub?.main?.state === "red", cluster: cluster.id, branches: branches.map(({ owner: _o, ...rest }) => rest) });
    for (const branch of branches) if (branch.owner !== undefined) links.push({ from: branch.owner, to: `main:${view.root}`, kind: "line" });
  }

  return { stars, landmarks, links, clusters, dust: DUST.map((one) => ({ x: one.x * field.width, y: one.y * field.height, r: one.r })) };
}

/** The star nearest in the direction pressed, for the arrows: the cursor moves to what is there. */
export function nearest(stars: readonly Star[], from: string, key: "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"): string | undefined {
  const here = stars.find((one) => one.key === from);
  if (here === undefined) return undefined;
  const dx = key === "ArrowRight" ? 1 : key === "ArrowLeft" ? -1 : 0;
  const dy = key === "ArrowDown" ? 1 : key === "ArrowUp" ? -1 : 0;
  let best: { key: string; score: number } | undefined;
  for (const star of stars) {
    if (star.key === from) continue;
    const along = (star.x - here.x) * dx + (star.y - here.y) * dy;
    if (along <= 0) continue;
    const across = Math.abs((star.x - here.x) * dy) + Math.abs((star.y - here.y) * dx);
    const score = along + across * 2;
    if (best === undefined || score < best.score) best = { key: star.key, score };
  }
  return best?.key;
}

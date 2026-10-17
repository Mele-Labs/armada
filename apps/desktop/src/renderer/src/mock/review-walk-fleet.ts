// The mock Fleet's Natural Search and Walk: the same calls the real one answers (`shared/api/reviews.ts`), over the mock forge's
// pull requests. **A search is a run of kind `search` kept under his words** across every repository. **The place a change runs is
// found as Fleet finds it**: a deploy bot's comment or a deploy check on the pull request, then what is remembered for the
// repository and the area, else nothing. A held walk moves checking out, starting, ready, as Fleet reports it, and the walk window
// is the mock's stand-in (`pr-review-walk-window.ts`). State is per window.

import { refusedWith } from "@armada/protocol";
import type {
  PullRequestCheckRow,
  PullRequestEntry,
  PullRequestFile,
  PullReviewRow,
  RememberedPlace,
  ReviewRun,
  ReviewWalk,
  WalkPlaceFound,
  WalkTried,
} from "@armada/protocol";

import type { ReviewsApi } from "../../../shared/api/reviews";
import type { ReviewWalkNote } from "../../../shared/review-walk";
import { areaShown, placeWords, searchName } from "../pr-review/runs";
import { unanswered } from "./moment";
import { openReviewWalkWindow } from "./pr-review-walk-window";
import type { ReviewSeed } from "./review-seed";

/** How long a search takes to come to something, and a setup line to run, in the mock. */
export const SEARCH_MS = 2800;
export const SETUP_MS = 320;

/** What the mock knows of a pull request that finding a place reads. */
export type WalkFacts = { repository: string; conversation: readonly PullRequestEntry[]; checks: readonly PullRequestCheckRow[]; files: readonly PullRequestFile[]; branch: string };

/** The directory, two deep, that most of the pull request's files are in, and where files tie the one with more changed lines. */
export function areaOf(files: readonly PullRequestFile[]): string {
  const by = new Map<string, { files: number; lines: number }>();
  for (const file of files) {
    const parts = file.path.split("/");
    const area = parts.length > 2 ? parts.slice(0, 2).join("/") : (parts[0] ?? "");
    const one = by.get(area) ?? { files: 0, lines: 0 };
    by.set(area, { files: one.files + 1, lines: one.lines + file.additions + file.deletions });
  }
  return [...by.entries()].sort((a, b) => b[1].files - a[1].files || b[1].lines - a[1].lines)[0]?.[0] ?? "";
}

const covers = (kept: string, area: string): boolean => areaShown(area).startsWith(areaShown(kept));

const announced = (line: string): string | undefined => /(preview|deployed)[^\n]*?(https?:\/\/[^\s)>\]"']+)/i.exec(line)?.[2];

/** What the pull request itself says about where it runs: a preview a deploy bot posted, or the address of a passed deploy check. */
function fromPullRequest(facts: WalkFacts): { tried: WalkTried; address: string } | undefined {
  for (const entry of facts.conversation) {
    if (entry.kind !== "comment" || !entry.author.toLowerCase().endsWith("bot")) continue;
    for (const line of entry.body.split("\n")) {
      const address = announced(line);
      if (address !== undefined) return { tried: { source: "pull_request", found: true, said: line.trim(), by: entry.author }, address };
    }
  }
  for (const check of facts.checks) {
    const address = check.address?.trim();
    if (check.state === "passed" && address !== undefined && /^https?:\/\//.test(address) && /preview|deploy/i.test(check.name)) {
      return { tried: { source: "pull_request", found: true, said: `${check.name}: ${address}`, by: check.name }, address };
    }
  }
  return undefined;
}

type Deps = {
  seed: ReviewSeed;
  /** The row a search turned up. */
  rowOf: (key: string) => PullReviewRow | undefined;
  factsOf: (manifestId: string, number: number) => WalkFacts | undefined;
  /** A run moved, as `review_run.changed` says it. */
  told: (run: ReviewRun) => void;
};

type Calls = Pick<
  ReviewsApi,
  | "startReviewSearch"
  | "getReviewSearches"
  | "forgetReviewSearch"
  | "findWalkPlace"
  | "listWalkPlaces"
  | "saveWalkPlace"
  | "forgetWalkPlace"
  | "openReviewWalk"
  | "closeReviewWalk"
  | "onReviewWalk"
  | "showReviewWalk"
  | "onReviewWalkNote"
  | "onReviewWalkClosed"
>;

export function createMockSearchAndWalk({ seed, rowOf, factsOf, told }: Deps): Calls {
  const searches = new Map<string, ReviewRun>();
  const places: RememberedPlace[] = seed.places.map((one) => ({ ...one }));
  const walks = new Map<string, ReviewWalk>();
  const walkListeners = new Set<(walk: ReviewWalk) => void>();
  const noteListeners = new Set<(note: ReviewWalkNote) => void>();
  const closedListeners = new Set<(walkId: string) => void>();
  let ids = 0;

  const moved = (walk: ReviewWalk) => {
    walks.set(walk.id, walk);
    walkListeners.forEach((one) => one({ ...walk }));
  };
  const after = (run: () => void, ms: number) => void globalThis.setTimeout(run, ms);
  const newest = (): ReviewRun[] => [...searches.values()].sort((a, b) => b.started_at.localeCompare(a.started_at)).map((run) => ({ ...run }));

  return {
    startReviewSearch: async (start) => {
      const name = searchName(start.words);
      const kept = searches.get(name);
      if (kept !== undefined && (kept.state === "queued" || kept.state === "running" || (kept.state === "done" && start.again !== true))) return { ok: true, value: { ...kept } };
      const run: ReviewRun = { manifest_id: "*", number: 0, kind: "search", name, head: "", state: "queued", started_at: new Date().toISOString() };
      searches.set(name, run);
      after(() => {
        run.state = "running";
        told(run);
      }, 0);
      after(() => {
        const matches = (seed.finds.find((one) => new RegExp(one.when, "i").test(start.words))?.hits ?? []).flatMap((hit) => {
          const pull = rowOf(hit.key);
          return pull === undefined ? [] : [{ pull, reason: hit.reason }];
        });
        run.state = "done";
        run.finished_at = new Date().toISOString();
        run.result = { shape: "matches", matches };
        told(run);
      }, SEARCH_MS);
      return { ok: true, value: { ...run } };
    },
    getReviewSearches: async () => ({ ok: true, value: { runs: newest() } }),
    forgetReviewSearch: async (forget) => {
      searches.delete(searchName(forget.words));
      return { ok: true, value: { runs: newest() } };
    },
    findWalkPlace: async (manifestId, number) => {
      const facts = factsOf(manifestId, number);
      if (facts === undefined) return { ok: false, outcome: unanswered(`/walk_finds/${manifestId}/${number}`) };
      const area = areaOf(facts.files);
      const tried: WalkTried[] = [];
      const posted = fromPullRequest(facts);
      if (posted !== undefined) {
        const value: WalkPlaceFound = { area, tried: [posted.tried], place: { how: "deployed", address: posted.address } };
        return { ok: true, value };
      }
      tried.push({ source: "pull_request", found: false, said: "No preview link in the comments or the checks" });
      const kept = places.find((one) => one.manifest_id === manifestId && covers(one.area, area));
      if (kept !== undefined) {
        const value: WalkPlaceFound = { area, tried: [...tried, { source: "memory", found: true, said: placeWords(kept.place), by: kept.area }], place: kept.place };
        return { ok: true, value };
      }
      tried.push({ source: "memory", found: false, said: `Nothing remembered for ${areaShown(area)} in ${facts.repository}` });
      return { ok: true, value: { area, tried } };
    },
    listWalkPlaces: async () => ({ ok: true, value: { places: places.map((one) => ({ ...one })) } }),
    saveWalkPlace: async (manifestId, save) => {
      const facts = [...seed.prs].find((one) => one.manifestId === manifestId);
      const at = places.findIndex((one) => one.manifest_id === manifestId && one.area === save.area);
      const next: RememberedPlace = { id: at >= 0 ? places[at]!.id : `place${(ids += 1)}`, manifest_id: manifestId, repository: facts?.repository ?? manifestId, area: save.area, place: save.place };
      if (at >= 0) places[at] = next;
      else places.push(next);
      return { ok: true, value: { ...next } };
    },
    forgetWalkPlace: async (manifestId, id) => {
      const at = places.findIndex((one) => one.manifest_id === manifestId && one.id === id);
      if (at < 0) return { ok: false, outcome: refusedWith(404, JSON.stringify({ code: "fleet.walk_place_unknown", message: "No such place is remembered" }), { method: "POST", path: `/walk_places/${manifestId}/${id}/forget` }) };
      places.splice(at, 1);
      return { ok: true, value: { places: places.map((one) => ({ ...one })) } };
    },
    openReviewWalk: async (manifestId, open) => {
      const facts = factsOf(manifestId, open.number);
      if (facts === undefined) return { ok: false, outcome: unanswered(`/review_walks/${manifestId}`) };
      const pr = seed.prs.find((one) => one.manifestId === manifestId && one.number === open.number);
      const slot = pr === undefined ? undefined : seed.walks[pr.key]?.slot;
      const address = open.place.address ?? open.place.api ?? "";
      const pulled = open.place.how === "pull";
      const walk: ReviewWalk = { id: `walk${(ids += 1)}`, manifest_id: manifestId, number: open.number, state: pulled ? "checking_out" : "ready", ...(pulled && slot !== undefined ? { slot } : {}), address, ...(open.place.api === undefined ? {} : { api: open.place.api }) };
      walks.set(walk.id, walk);
      if (pulled) {
        after(() => moved({ ...walk, state: "starting" }), SETUP_MS);
        after(() => moved({ ...walk, state: "ready" }), SETUP_MS * 3);
      }
      return { ok: true, value: { ...walk } };
    },
    closeReviewWalk: async (_manifestId, id) => {
      const walk = walks.get(id);
      if (walk === undefined) return { ok: false, outcome: unanswered(`/review_walks/${id}`) };
      const closed: ReviewWalk = { ...walk, state: "closed" };
      moved(closed);
      return { ok: true, value: closed };
    },
    onReviewWalk: (onWalk) => {
      walkListeners.add(onWalk);
      return () => walkListeners.delete(onWalk);
    },
    showReviewWalk: async (shown) => {
      if (shown.address === "") return { ok: false, why: "no_address" };
      if ((shown.steps ?? shown.requests ?? []).length === 0) return { ok: false, why: "no_steps" };
      const pr = seed.prs.find((one) => one.manifestId === shown.manifestId && one.number === shown.number);
      const walk = pr === undefined ? undefined : seed.walks[pr.key];
      const runner =
        shown.requests === undefined
          ? undefined
          : { base: shown.address, requests: shown.requests.map((request, at) => ({ request, response: (walk?.kind === "api" ? walk.responses[at] : undefined) ?? { status: 200, ms: 20, body: "{}" } })) };
      openReviewWalkWindow(shown, runner, {
        onNote: (text, location) => noteListeners.forEach((one) => one({ manifestId: shown.manifestId, number: shown.number, at: new Date().toISOString(), text, location })),
        onClose: () => {
          const held = walks.get(shown.walkId);
          if (held !== undefined && held.state !== "closed") moved({ ...held, state: "closed" });
          closedListeners.forEach((one) => one(shown.walkId));
        },
      });
      return { ok: true };
    },
    onReviewWalkNote: (onNote) => {
      noteListeners.add(onNote);
      return () => noteListeners.delete(onNote);
    },
    onReviewWalkClosed: (onClosed) => {
      closedListeners.add(onClosed);
      return () => closedListeners.delete(onClosed);
    },
  };
}

/** What a window with no mock forge answers to the same calls: that nothing is behind it. */
export const NO_SEARCH_OR_WALK: Calls = {
  startReviewSearch: async () => ({ ok: false, outcome: unanswered("/review_searches") }),
  getReviewSearches: async () => ({ ok: false, outcome: unanswered("/review_searches") }),
  forgetReviewSearch: async () => ({ ok: false, outcome: unanswered("/review_searches/forget") }),
  findWalkPlace: async (manifestId, number) => ({ ok: false, outcome: unanswered(`/walk_finds/${manifestId}/${number}`) }),
  listWalkPlaces: async () => ({ ok: false, outcome: unanswered("/walk_places") }),
  saveWalkPlace: async (manifestId) => ({ ok: false, outcome: unanswered(`/walk_places/${manifestId}`) }),
  forgetWalkPlace: async (manifestId, id) => ({ ok: false, outcome: unanswered(`/walk_places/${manifestId}/${id}/forget`) }),
  openReviewWalk: async (manifestId) => ({ ok: false, outcome: unanswered(`/review_walks/${manifestId}`) }),
  closeReviewWalk: async (manifestId, id) => ({ ok: false, outcome: unanswered(`/review_walks/${manifestId}/${id}/close`) }),
  onReviewWalk: () => () => undefined,
  showReviewWalk: async () => ({ ok: false, why: "no_address" }),
  onReviewWalkNote: () => () => undefined,
  onReviewWalkClosed: () => () => undefined,
};

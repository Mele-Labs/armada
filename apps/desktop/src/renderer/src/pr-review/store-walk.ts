// **The Walk tool: where a change runs, the place held up for a walk, and the window it plays in.** The walk itself is a run of
// the tool `walk` (`store-agent.ts` starts it, with the place), kept for the commit it was written on. This half finds the place
// first (`find_walk_place`: the pull request, then what Armada remembers, reading and spending nothing), asks him where nothing
// says, remembers his answer for the area, and on Walk it holds the place up (`open_review_walk`) and opens Armada's own walk window
// on it (`showReviewWalk`). A note taken in that window where no Session holds the pull request stays here and is never posted.

import type { RememberedPlace, ReviewWalk, WalkPlace } from "@armada/protocol";

import type { ReviewSnapshot, WalkWork } from "./model";
import { runKey, walkOf } from "./runs";
import type { ReviewSource } from "./source";
import type { AgentActs, StoreHold } from "./store-agent";
import type { ReviewWalkNote, ShownReviewWalkOutcome } from "../../../shared/review-walk";

/** The id of the tool whose run is the walk. */
export const WALK_TOOL = "walk";

/** The key of the walk's run in `PrWork.wire`. */
export const WALK_RUN = runKey("tool", WALK_TOOL);

export type WalkActs = {
  /** Finds where this pull request runs, then writes the walk against it, or asks him. A walk kept already is written over. */
  findPlace: (key: string) => void;
  /** His answer to the question, remembered for `area` where one is given. Writes the walk against it. */
  answer: (key: string, place: WalkPlace, area: string | undefined) => void;
  /** Puts the question up again, after Esc left it open on the page. */
  askWhere: (key: string) => void;
  closeSheet: () => void;
  /** Opens the sheet of places Armada remembers, and reads them. */
  showPlaces: () => void;
  readPlaces: () => void;
  /** Changes where a remembered place runs. A refusal is Fleet's words, and the list stays. */
  editPlace: (one: RememberedPlace, place: WalkPlace) => void;
  forgetPlace: (one: RememberedPlace) => void;
  /** Holds the place up and opens the walk window on the walk this pull request has. */
  walkIt: (key: string) => void;
  /** A held walk moved, whole (`review_walk.changed`). */
  applyWalk: (walk: ReviewWalk) => void;
  /** A note taken in a walk window that no Session holds. */
  noted: (note: ReviewWalkNote) => void;
  /** A walk window closed, and its walk with it. */
  closed: (walkId: string) => void;
};

const WHY: Record<Extract<ShownReviewWalkOutcome, { ok: false }>["why"], string> = {
  no_address: "The walk has no address to open",
  no_steps: "The walk has no steps",
  runner_failed: "The page for the requests did not start",
};

export function createWalkActs(source: ReviewSource, hold: StoreHold, agent: AgentActs): WalkActs {
  const { get, set, work, change, prOf } = hold;
  /** The walks whose window was opened, so a walk that is ready twice opens one. */
  const shown = new Set<string>();

  const setWalk = (key: string, by: Partial<WalkWork>) => change(key, { walk: { ...work(key).walk, ...by } });
  const keyOf = (manifestId: string, number: number): string | undefined => get().prs.find((one) => one.manifestId === manifestId && one.number === number)?.key;

  /** Writes the walk, over a kept one where there is one. */
  const write = (key: string, place: WalkPlace) => {
    const kept = work(key).runs[WALK_TOOL];
    agent.begin(key, "tool", WALK_TOOL, kept !== undefined && kept.state !== "idle", place);
  };

  const show = (key: string, walk: ReviewWalk) => {
    const pr = prOf(key);
    const written = walkOf(work(key).wire[WALK_RUN]);
    if (pr === undefined || written === undefined || shown.has(walk.id)) return;
    shown.add(walk.id);
    void source
      .showWalk({
        walkId: walk.id,
        manifestId: pr.manifestId,
        number: pr.number,
        title: `#${pr.number} ${pr.title}`,
        address: walk.address,
        ...(written.shape === "web_walk" ? { steps: written.steps } : { requests: written.requests }),
        ...(pr.holder === undefined ? {} : { holder: { id: pr.holder.id, title: pr.holder.title } }),
      })
      .then((outcome) => {
        if (outcome.ok) return setWalk(key, { opening: false, said: undefined });
        shown.delete(walk.id);
        setWalk(key, { opening: false, said: WHY[outcome.why] });
      });
  };

  const applyWalk = (walk: ReviewWalk) => {
    const key = keyOf(walk.manifest_id, walk.number);
    if (key === undefined) return;
    const was = work(key).walk;
    if (walk.state === "closed") {
      shown.delete(walk.id);
      return setWalk(key, { held: undefined, opening: false });
    }
    if (walk.state === "failed") {
      return setWalk(key, { held: undefined, opening: false, reached: was.held?.state, said: walk.failed ?? "The walk did not start" });
    }
    setWalk(key, { held: walk });
    if (walk.state === "ready" && was.opening) show(key, walk);
  };

  const places = (by: Partial<ReviewSnapshot["places"]>) => set({ places: { ...get().places, ...by } });

  return {
    findPlace: (key) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      change(key, { view: WALK_TOOL });
      setWalk(key, { find: { state: "looking" }, asking: false, said: undefined, reached: undefined });
      void source.findPlace(pr.manifestId, pr.number).then((answer) => {
        if (prOf(key) === undefined) return;
        if (!answer.ok) return setWalk(key, { find: { state: "said", said: answer.said } });
        const found = answer.value;
        setWalk(key, { find: { state: "found", found } });
        if (found.place !== undefined) return write(key, found.place);
        setWalk(key, { asking: true });
        set({ sheet: { kind: "ask", key } });
      });
    },
    answer: (key, place, area) => {
      const pr = prOf(key);
      const find = work(key).walk.find;
      if (pr === undefined || find.state !== "found") return;
      set({ sheet: undefined });
      setWalk(key, { asking: false, find: { state: "found", found: { ...find.found, place } } });
      write(key, place);
      if (area === undefined || area === "") return;
      void source.savePlace(pr.manifestId, { area, place }).then((saved) => {
        if (!saved.ok) setWalk(key, { said: saved.said });
        else if (get().places.state === "read") places({ list: [...get().places.list.filter((one) => one.id !== saved.value.id), saved.value] });
      });
    },
    askWhere: (key) => set({ sheet: { kind: "ask", key } }),
    closeSheet: () => set({ sheet: undefined }),
    showPlaces: () => {
      set({ sheet: { kind: "places" } });
      if (get().places.state !== "reading") {
        places({ state: "reading", said: undefined });
        void source.places().then((answer) => (answer.ok ? places({ state: "read", list: answer.value.places, said: undefined }) : places({ state: "read", said: answer.said })));
      }
    },
    readPlaces: () => {
      if (get().places.state !== "unread") return;
      places({ state: "reading" });
      void source.places().then((answer) => (answer.ok ? places({ state: "read", list: answer.value.places }) : places({ state: "unread", said: answer.said })));
    },
    editPlace: (one, place) => {
      void source.savePlace(one.manifest_id, { area: one.area, place }).then((answer) => {
        if (!answer.ok) return places({ said: answer.said });
        places({ said: undefined, list: get().places.list.map((it) => (it.id === one.id ? answer.value : it)) });
      });
    },
    forgetPlace: (one) => {
      void source.forgetPlace(one.manifest_id, one.id).then((answer) => (answer.ok ? places({ said: undefined, list: answer.value.places }) : places({ said: answer.said })));
    },
    walkIt: (key) => {
      const pr = prOf(key);
      const written = walkOf(work(key).wire[WALK_RUN]);
      const current = work(key).walk;
      if (pr === undefined || written === undefined || current.opening) return;
      setWalk(key, { opening: true, said: undefined, reached: undefined });
      void source.openWalk(pr.manifestId, { number: pr.number, place: written.place }).then((answer) => {
        if (!answer.ok) return setWalk(key, { opening: false, said: answer.said });
        applyWalk(answer.value);
      });
    },
    applyWalk,
    noted: (note) => {
      const key = keyOf(note.manifestId, note.number);
      if (key === undefined) return;
      setWalk(key, { notes: [...work(key).walk.notes, { at: note.at, text: note.text, location: note.location }] });
    },
    closed: (walkId) => {
      shown.delete(walkId);
      for (const [key, one] of Object.entries(get().work)) {
        if (one.walk.held?.id === walkId) setWalk(key, { held: undefined, opening: false });
      }
    },
  };
}

// **Natural Search: one run across every repository, kept by Fleet under his words.** He writes what he wants, and on send (after
// `spend` has asked) Fleet starts a run of kind `search` that answers the open pull requests that matched, each with one line
// on why. A search is a list of its own in the dropdown, kept with when it ran and never marked out of date, run again only
// when he asks, and dropped at Fleet when he removes it, so it is gone after a restart. A search has no pull request, so
// the store routes it here before the by-pull-request lookup (`store.tsx`).

import type { ReviewRun } from "@armada/protocol";

import type { Find } from "./model";
import { agentRunOf, matchesOf, searchId, searchName, wordsOf } from "./runs";
import type { ReviewSource } from "./source";
import { staler, type StoreHold } from "./store-agent";

export type SearchActs = {
  /** Sends what he wrote: Fleet answers the search kept for these words as it stands, or starts one. */
  find: (words: string) => void;
  /** Runs it again, whatever is kept. */
  rerunFind: (id: string) => void;
  /** Drops it at Fleet, then from the dropdown. A refusal is Fleet's words, and it stays. */
  removeFind: (id: string) => void;
  /** Reads every search kept or going. Called when the tab opens and on Refresh, since the event is not replayed. */
  readSearches: () => void;
  /** A search moved, whole (`review_run.changed`). */
  applySearch: (run: ReviewRun) => void;
};

export function createSearchActs(source: ReviewSource, hold: StoreHold): SearchActs {
  const { get, set, adopt } = hold;
  /** His words as he wrote them, by Fleet's name for them: the dropdown shows these while this window has them. */
  const typed = new Map<string, string>();
  /** The runs as Fleet last said them, so an older start arriving late does not take a finished search back. */
  const wire = new Map<string, ReviewRun>();
  /** Searches he removed: they stay off until he asks the same again, whatever a late event says. */
  const dropped = new Set<string>();

  const upsert = (next: Find) => {
    const finds = get().finds;
    set({ finds: finds.some((one) => one.id === next.id) ? finds.map((one) => (one.id === next.id ? next : one)) : [...finds, next] });
  };

  const applySearch = (run: ReviewRun) => {
    if (dropped.has(run.name)) return;
    const was = wire.get(run.name);
    if (was !== undefined && staler(was, run)) return;
    wire.set(run.name, run);
    const hits = matchesOf(run).map((one) => ({ key: adopt(one.pull), reason: one.reason }));
    upsert({ id: searchId(run.name), words: typed.get(run.name) ?? run.name, run: agentRunOf(run), hits });
  };

  const begin = (name: string, words: string, again: boolean) => {
    const id = searchId(name);
    upsert({ id, words, run: { state: "running", since: Date.now() }, hits: [] });
    void source.startSearch({ words, ...(again ? { again: true } : {}) }).then((answer) => {
      if (answer.ok) return applySearch(answer.value);
      const one = get().finds.find((it) => it.id === id);
      if (one !== undefined) upsert({ ...one, run: { state: "failed", said: answer.said, at: Date.now(), head: "" } });
    });
  };

  return {
    find: (words) => {
      const asked = wordsOf(words);
      if (asked === "") return;
      const name = searchName(asked);
      typed.set(name, asked);
      dropped.delete(name);
      const had = get().finds.find((one) => one.id === searchId(name));
      set({ tab: searchId(name) });
      if (had?.run.state !== "running") begin(name, asked, false);
    },
    rerunFind: (id) => {
      const one = get().finds.find((it) => it.id === id);
      if (one !== undefined && one.run.state !== "running") begin(searchName(one.words), one.words, true);
    },
    removeFind: (id) => {
      const one = get().finds.find((it) => it.id === id);
      if (one === undefined || one.run.state === "running") return;
      void source.forgetSearch({ words: one.words }).then((answer) => {
        if (!answer.ok) return set({ failed: answer.said });
        const name = searchName(one.words);
        dropped.add(name);
        wire.delete(name);
        set({ finds: get().finds.filter((it) => it.id !== id), tab: get().tab === id ? "requested" : get().tab, failed: undefined });
      });
    },
    readSearches: () => {
      void source.searches().then((answer) => {
        if (!answer.ok) return;
        for (const run of [...answer.value.runs].reverse()) applySearch(run);
      });
    },
    applySearch,
  };
}

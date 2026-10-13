// Helm's message box is the Sessions box. The same three repositories, Sessions and Jobs as
// `route-a-thought`, with Helm answering a question about two pull requests: the box brings files,
// pictures and sketches, `/`, `@` and `#`, and what it sent shows in the thread. Over the walk
// `helm-rich-composer`. The issues and pull requests `#` offers are `mock/numbered.ts`.

import type { HelmThreadItem, MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import type { Scenario } from "../moment";
import { s210RouteAThought } from "./route-a-thought";

/** Helm's reply, in the facts of the two pull requests the walk tags and nothing past them. */
const REPLY = [
  "#219 moves the Bridge store onto one clock, and #207 stopped the Drone count drifting. Neither touches the Sessions page.",
  "Sessions list views holds #216, and Job 12 stopped on a gate.",
].join("\n\n");

const FORGE = "https://forge.invalid/pull/";
const pull = (number: number, title: string, branch: string) => ({ number, title, branch, url: `${FORGE}${number}`, ci: "passed" });

/** armada's merge line: three pull requests open, and #207 landed. Where a pull request named in Helm's thread opens. */
const LINE: MergeLine = {
  root: repository().root,
  line: [],
  off: [],
  landed: [{ branch: "fix/drone-count", pull_request: { number: 207, url: `${FORGE}207`, settled: "merged" }, state: "landed", merge_commit: "9c1d0b7e4a2f6d83b50c9e71a4f2d6b8c3e5a107" }],
  sent_back: [],
  hub: {
    pull_requests: [pull(219, "Pin the Bridge store clock", "fix/store-clock"), pull(216, "Fold the two notification routes into one", "notify/one-route"), pull(203, "Sessions page: Active, Quiet and Ended views", "sessions/list-views")],
  },
};

const now = () => new Date().toISOString();

export const s211HelmRichComposer: Scenario = {
  ...s210RouteAThought,
  name: "helm-rich-composer",
  says: "Helm's message box is the Sessions box: files, sketches, /, @ and #, in three repositories",
  state: { ...s210RouteAThought.state, mergeLines: { lines: [LINE] } },
  behaves: (handle) => ({
    ...s210RouteAThought.behaves!(handle),
    askHelm: async (text) => {
      const at = now();
      const held = handle.state().helm;
      const was: HelmThreadItem[] = held.state === "open" ? held.items : [];
      const item = (saw: HelmThreadItem): HelmThreadItem => saw;
      handle.publish({
        helm: {
          state: "open",
          manifestId: "armada",
          replying: false,
          skipped: 0,
          missed: 0,
          items: [
            ...was,
            item({ kind: "asked", id: String(was.length), ts: at, text }),
            item({ kind: "row", id: String(was.length + 1), turn: { ts: at, seq: was.length + 1, by: "drone", saw: { event: "said", text: REPLY } } }),
            item({ kind: "row", id: String(was.length + 2), turn: { ts: at, seq: was.length + 2, by: "drone", saw: { event: "ended", turns: 1, cost_micros: 6_100, refusals: 0 } } }),
          ],
        },
      });
      return { ok: true };
    },
  }),
};

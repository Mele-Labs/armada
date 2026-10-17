// Helm's message box is the Sessions box. The same three repositories, Sessions and Jobs as
// `route-a-thought`, with Helm answering a question about two pull requests: the box brings files,
// pictures and sketches, `/`, `@` and `#`, and what it sent shows in the thread. Over the walk
// `helm-rich-composer`. The issues and pull requests `#` offers are `mock/numbered.ts`.

import type { MergeLine } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { helmFleetOn } from "../helm-carry";
import { s210RouteAThought } from "./route-a-thought";

/** Helm's reply, in the facts of the two pull requests the walk tags and nothing past them. */
const REPLY = [
  "#2219 moves the Bridge store onto one clock, and #2207 stopped the Drone count drifting. Neither touches the Sessions page.",
  "Sessions list views holds #2216, and Job 12 stopped on a gate.",
].join("\n\n");

const FORGE = "https://forge.invalid/pull/";
const pull = (number: number, title: string, branch: string) => ({ number, title, branch, url: `${FORGE}${number}`, ci: "passed" });

/** armada's merge line: three pull requests open, and #2207 landed. Where a pull request named in Helm's thread opens. */
const LINE: MergeLine = {
  root: repository().root,
  line: [],
  off: [],
  landed: [{ branch: "fix/drone-count", pull_request: { number: 2207, url: `${FORGE}2207`, settled: "merged" }, state: "landed", merge_commit: "9c1d0b7e4a2f6d83b50c9e71a4f2d6b8c3e5a107" }],
  sent_back: [],
  hub: {
    pull_requests: [pull(2219, "Pin the Bridge store clock", "fix/store-clock"), pull(2216, "Fold the two notification routes into one", "notify/one-route"), pull(203, "Sessions page: Active, Quiet and Ended views", "sessions/list-views")],
  },
};

export const s211HelmRichComposer: Scenario = {
  ...s210RouteAThought,
  name: "helm-rich-composer",
  says: "Helm's message box is the Sessions box: files, sketches, /, @ and #, in three repositories",
  state: { ...s210RouteAThought.state, mergeLines: { lines: [LINE] } },
  // What the message carried is kept on its row, as Fleet keeps it, and Helm's reply follows.
  behaves: (handle) => helmFleetOn(s210RouteAThought, handle, REPLY),
};

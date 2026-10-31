// A Job's area in panes the person arranges: five Jobs on the Cockpit, each with a story, a shell to open in a Terminal tab and a browser to open in a Browser tab. Two layouts are saved
// already, QA split and Review, beside Default. Each time the walk lets time pass, the second Job opens a browser page itself, which joins the pane that holds a browser, or opens
// as a split on the right. Over `job-story`. The walk `jobLayouts` plays it. Mock only: nothing is kept.

import { seedJobBrowser } from "../../job-browser/store";
import { jobOpens } from "../../job-panes/store";
import { resetCheckout, shopPage } from "../job-browser-page";
import type { Scenario } from "../moment";
import { sJobStory } from "./job-story";
import { disposeShells, shellApi } from "./session-shell";

/** The Jobs `job-story` serves. */
const JOBS = ["story-1", "story-2", "story-3", "story-4", "story-5"];
/** The Job that opens a page by itself. */
const OPENS = "story-2";
/** The Job with changes to choose between, and the Job it started. */
const CHANGES = "story-1";
const SUB = "story-4";
const SHOP = ["http://localhost:5173/cart", "http://localhost:5173/checkout"];

export const sJobLayouts: Scenario = {
  ...sJobStory,
  name: "job-layouts",
  says: "A Job's area in panes: split, tabs of any kind, saved layouts, and a page the Job opens by itself",
  behaves: (fleet) => {
    resetCheckout();
    for (const id of JOBS) seedJobBrowser(id, { visited: [...SHOP], page: shopPage });
    return { ...sJobStory.behaves?.(fleet), ...shellApi() };
  },
  draft: {
    ...sJobStory.draft,
    sessions: (board, more) => {
      const store = sJobStory.draft!.sessions!(board, more);
      // The first Job has changes from three places, so a Changes tab has a choice: its branch, two pull requests of its own, and the Job it started, which has a pull request.
      store.edit(CHANGES, (one) => ({
        ...one,
        attachments: [
          ...one.attachments,
          { kind: "branch", name: "armada/retry-lease", slot: 3 },
          { kind: "pull_request", number: 2390, title: "Retry the slot lease", branch: "armada/retry-lease", address: "https://git.example/armada/pull/2390", checks: { state: "passed" }, state: "open", auto: false },
          { kind: "pull_request", number: 2391, title: "Name the attempt in the bay tooltip", branch: "armada/retry-lease-tooltip", address: "https://git.example/armada/pull/2391", checks: { state: "pending" }, state: "draft", auto: false },
          { kind: "started_to", id: SUB },
        ],
      }));
      return {
        ...store,
        terminalRuns: true as const,
        later: () => {
          store.later();
          // The tab reads the host of the page the Job opened, on the dev server's other port; the mock's browser is the one it already has.
          jobOpens(OPENS, "browser", "localhost:5174");
        },
        dispose: () => {
          disposeShells();
          store.dispose();
        },
      };
    },
  },
};

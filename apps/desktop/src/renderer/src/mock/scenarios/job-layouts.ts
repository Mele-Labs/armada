// A Job's area in panes the person arranges: five Jobs on the Cockpit, each with a story, a shell to open in a Terminal tab and a browser to open in a Browser tab. Two layouts are saved
// already, QA split and Review, beside Default. Each time the walk lets time pass, the second Job opens a browser page itself, which joins the pane that holds a browser, or opens
// as a split on the right. Over `job-story`. The walk `jobLayouts` plays it. Mock only: nothing is kept.

import type { WorktreeSlot } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { seedJobBrowser } from "../../job-browser/store";
import { jobOpens } from "../../job-panes/store";
import { resetCheckout, shopPage } from "../job-browser-page";
import type { Scenario } from "../moment";
import { sJobStory } from "./job-story";
import { seedSystem } from "../slices/session-system";
import { disposeShells, shellApi } from "./session-shell";

/** The Jobs `job-story` serves. */
const JOBS = ["story-1", "story-2", "story-3", "story-4", "story-5"];
/** The Job that opens a page by itself. */
const OPENS = "story-2";
/** The Job with changes to choose between, and the Job it started. */
const CHANGES = "story-1";
const SUB = "story-4";
const KB = 1024;
const GB = KB * KB * KB;
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
  // The bay the first Job holds, so its Bays and System tabs have something to draw.
  held: {
    ...sJobStory.held,
    slots: [
      ...(sJobStory.held?.slots ?? []),
      {
        manifest_id: repository().manifest!.id,
        slot: 3,
        path: "/Users/user/armada/.armada/slots/slot-3",
        base: "main",
        warm: true,
        behind: 0,
        held: { state: "hosted_session", session_id: CHANGES, session_title: "Retry the slot lease" },
        branch: "armada/retry-lease",
      } satisfies WorktreeSlot,
    ],
  },
  draft: {
    ...sJobStory.draft,
    sessions: (board, more) => {
      seedSystem({
        session_id: CHANGES,
        processes: [
          { pid: 48211, command: "vite --port 5173", port: 5173, state: "running" },
          { pid: 47802, command: "vitest watch", state: "stopped" },
        ],
        cpu_percent: 38,
        memory_bytes: 1.8 * GB,
        memory_of_bytes: 8 * GB,
        disk_bytes: 6.4 * GB,
        disk_of_bytes: 20 * GB,
        worktrees: [],
        logs: [{ name: "session.log", path: `/mock/keepers/${CHANGES}.log`, bytes: 96 * KB }],
      });
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
          { kind: "artifact", form: "doc", id: "docs/design/lease-retry.md", title: "How the lease retries" },
          { kind: "artifact", form: "page", id: "http://localhost:5173/checkout", title: "The checkout page" },
          { kind: "artifact", form: "file", id: "crates/fleet/src/lease.rs", title: "lease.rs" },
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

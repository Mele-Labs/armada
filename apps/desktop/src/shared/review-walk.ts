// What a review's walk window is asked to show, and the names Bridge's main and its renderer must agree on. **An API walk
// plays in the same window a Session's `show_window` opens**: its page is a runner of Armada's own (`main/capture/runner.ts`)
// with a button for each request, and its steps press those buttons, so the window has one way to walk anything. The mock
// draws the same page in the browser (`renderer/src/pr-review/walk-runner.tsx`), which is why these live in `shared`.

import type { ReviewRequest, WalkStep } from "@armada/protocol";

/** What `showReviewWalk` hands main. `steps` for a web walk, `requests` for an API walk, and exactly one of the two. */
export type ShownReviewWalk = {
  /** The held walk's id, which main closes at Fleet when the window closes. */
  walkId: string;
  manifestId: string;
  number: number;
  /** The window's name: `#12 Take a draft out`. */
  title: string;
  /** Where the change runs: the address a web walk loads, or the API's base address an API walk sends to. */
  address: string;
  steps?: readonly WalkStep[];
  requests?: readonly ReviewRequest[];
  /** The Session of his that holds the pull request, where one does. A note taken in the window goes to it; without one it stays in Armada. */
  holder?: { id: string; title: string };
};

/** What `showReviewWalk` answers. */
export type ShownReviewWalkOutcome = { ok: true } | { ok: false; why: "no_address" | "no_steps" | "runner_failed" };

/** A note taken in a review's walk window where no Session holds the pull request: it stays in Armada, and is never a comment on the forge. */
export type ReviewWalkNote = { manifestId: string; number: number; at: string; text: string; location: string };

/** The name of the button that sends a request on the runner page, which is what the request's step presses. */
export const sendName = (request: ReviewRequest, at: number): string => `Send ${at + 1}: ${request.method} ${request.path}`;

/** An API walk as steps: one per request, pressing its send. **The step's words are what to look for**; the page shows the request and its response. */
export const stepsOfRequests = (requests: readonly ReviewRequest[]): WalkStep[] =>
  requests.map((one, at) => ({ say: `${one.say}. Look for: ${one.look}`, press: { role: "button", name: sendName(one, at), exact: true } }));

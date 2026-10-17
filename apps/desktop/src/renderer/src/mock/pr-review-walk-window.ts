// What opens Armada's walk window for a review's walk in the mock: **the stand-in the mock already opens when a Session
// shows a page** (`openWalkWindow`, `@armada/jobs/fake`), with the same bar, steps, Autoplay, notes and keys. The page in its
// frame is the mock itself, since the preview a pull request deployed is not reachable from a browser tab: it plays the
// walk's steps, handed over in the address as `show_window` hands Bridge its `steps` (`inline-walk.ts`). An API walk's
// page is the request runner (`review-walk-runner.tsx`), which the same steps press.

import { mockPage, openWalkWindow } from "@armada/jobs/fake";

import type { ShownReviewWalk } from "../../../shared/review-walk";
import { stepsOfRequests } from "../../../shared/review-walk";
import type { RunnerPage } from "./review-walk-runner";

export type WalkWindowHooks = {
  /** A note taken in the window, where no Session holds the pull request. */
  onNote: (text: string, location: string) => void;
  onClose: () => void;
};

/** Opens the stand-in on a walk. `runner` is the page of an API walk: its requests with the response each is sent for. */
export function openReviewWalkWindow(shown: ShownReviewWalk, runner: RunnerPage | undefined, hooks: WalkWindowHooks): void {
  const steps = shown.steps ?? stepsOfRequests(shown.requests ?? []);
  const url = new URL(mockPage());
  url.search = "";
  url.searchParams.set("scenario", "bays-look-like-bays");
  url.searchParams.set("steps", JSON.stringify(steps));
  if (runner !== undefined) url.searchParams.set("runner", JSON.stringify(runner));
  url.searchParams.set("walked", "");
  openWalkWindow(shown.title, url.toString(), {
    title: shown.title,
    address: shown.address,
    // A note goes to the Session that holds the pull request, as main sends it. Without one it stays in Armada.
    job: shown.holder?.title ?? "Armada",
    into: "Session",
    onNote: (said, picked) => (shown.holder === undefined ? hooks.onNote(said, picked.location) : undefined),
    readiness: "ready",
    onClose: hooks.onClose,
  });
}

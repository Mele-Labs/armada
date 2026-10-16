// What opens Armada's walk window for a review's walk in the mock: **the stand-in the mock already opens when a Session
// shows a page** (`openWalkWindow`, `@armada/jobs/fake`), with the same bar, steps, Autoplay, notes and keys. The page in its
// frame is the mock itself, since the preview a pull request deployed is not reachable from a browser tab: it plays the
// walk's steps, handed over in the address as `show_window` hands Bridge its `steps` (`inline-walk.ts`). An API walk's
// page is the request runner (`pr-review/walk-runner.tsx`), which the same steps press.

import { mockPage, openWalkWindow } from "@armada/jobs/fake";

import type { WalkOpen } from "../pr-review/walk-model";

export const openReviewWalk: WalkOpen = (shown) => {
  const url = new URL(mockPage());
  url.search = "";
  url.searchParams.set("scenario", "bays-look-like-bays");
  url.searchParams.set("steps", JSON.stringify(shown.steps));
  if (shown.runner !== undefined) url.searchParams.set("runner", JSON.stringify(shown.runner));
  url.searchParams.set("walked", "");
  openWalkWindow(shown.title, url.toString(), {
    title: shown.title,
    address: shown.address,
    job: shown.title,
    into: "Session",
    // A note goes to the Session that holds the pull request. Without one it has nowhere to go.
    onNote: (said, picked) => shown.tell?.(`${said}\nOn ${picked.element}, ${picked.location}`),
    readiness: "ready",
  });
};

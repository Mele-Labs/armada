// Natural Search and the Walk tool: where a change runs, and the place held up for a walk.
// `crates/ipc/src/review_walks.rs`, `docs/concepts/pull-review.md`, *Natural Search* and *Walk*.
//
// The header rules in `protocol.ts` hold here.

import type { PullReviewRow } from "./pull-reviews";

/** The `manifest_id` and `number` a search run is kept under: a search spans every repository, so it belongs to none. Its `head` is empty and its `name` the words, lower cased and single spaced. */
export const SEARCH_RUN_SCOPE = "*";

/** `start_review_search`'s body (`POST /review_searches`). */
export type StartReviewSearch = {
  /** What the person wants, in their own words. */
  words: string;
  /** Run it again, whatever is kept. Without it a kept search of the same words is answered as it stands. */
  again?: boolean;
};

/** `forget_review_search`'s body (`POST /review_searches/forget`). */
export type ForgetReviewSearch = {
  /** The words of the search to drop, as he asked them. */
  words: string;
};

/** One pull request a search turned up, and the one line that says why. Always an open pull request of a served repository. */
export type ReviewMatch = {
  pull: PullReviewRow;
  reason: string;
};

/** How a change is reached: already running somewhere, or pulled down into a slot and started there. */
export type WalkHow = "deployed" | "pull";

/** Where a change runs. At least one of `address` and `api` is set; for `pull`, `command` is too. */
export type WalkPlace = {
  how: WalkHow;
  /** The web address, an http or https one. */
  address?: string;
  /** What starts the change, run in the pulled-down checkout. Only for `pull`. */
  command?: string;
  /** The API's base address, an http or https one, where the change has an API. */
  api?: string;
};

/** Where Fleet looked for a place: the pull request itself, then what it remembers of the repository and the area. */
export type WalkSource = "pull_request" | "memory";

/** One source tried and what it came to. Asking the person is the third source and is Bridge's. */
export type WalkTried = {
  source: WalkSource;
  found: boolean;
  /** The bot's line, the check's name and address, a remembered place's words, or what was not there. */
  said: string;
  /** The bot's login, the check's name, or the area a memory is for. */
  by?: string;
};

/** `find_walk_place` (`GET /walk_finds/:repository/:number`): reads and spends nothing. */
export type WalkPlaceFound = {
  /** The two-deep directory with the most changed files: what a place is remembered for. */
  area: string;
  tried: WalkTried[];
  /** Absent: nothing was found, and the person is asked. */
  place?: WalkPlace;
};

/** A place kept for one area of one repository. */
export type RememberedPlace = {
  id: string;
  manifest_id: string;
  /** `owner/name`. */
  repository: string;
  /** A directory, two deep at most, no trailing slash: `crates/fleet`. */
  area: string;
  place: WalkPlace;
};

/** `list_walk_places` (`GET /walk_places`), and what `forget_walk_place` answers. */
export type RememberedPlaces = {
  places: RememberedPlace[];
};

/** `save_walk_place`'s body (`POST /walk_places/:repository`). Replaces the place kept for the area. */
export type SaveWalkPlace = {
  area: string;
  place: WalkPlace;
};

/** `open_review_walk`'s body (`POST /review_walks/:repository`). */
export type OpenReviewWalk = {
  number: number;
  place: WalkPlace;
};

/** How far a walk's place has got. `failed` has already given its slot back. */
export type ReviewWalkState = "checking_out" | "starting" | "ready" | "failed" | "closed";

/** A place held up for a walk. Carried by `open_review_walk`, `close_review_walk` and `review_walk.changed`. */
export type ReviewWalk = {
  id: string;
  manifest_id: string;
  number: number;
  state: ReviewWalkState;
  /** The slot a pulled-down walk holds until it is closed. */
  slot?: number;
  /** What the walk window opens: the place's address, else its API base. */
  address: string;
  /** The API's base address, where there is one. */
  api?: string;
  /** Why a failed walk failed, in Fleet's words. */
  failed?: string;
};

/** A request an API walk may send. */
export type ReviewRequestMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

/** One request of an API walk, and what to look for in its response. */
export type ReviewRequest = {
  /** What the step says while it is on screen. */
  say: string;
  method: ReviewRequestMethod;
  /** A path on the API's base address: it starts with `/` and names no host. */
  path: string;
  /** The request body, JSON text. */
  body?: string;
  /** What to look for in the response. */
  look: string;
};

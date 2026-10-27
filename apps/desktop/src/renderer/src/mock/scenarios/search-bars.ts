// The search bar on System, Projects, Reviews and Issues, in one window: the Projects scenario's tasks and Issues tab, and the review
// surface's pull requests on top. The walk `search-bars` plays it.

import type { Scenario } from "../moment";
import { reviewSeed } from "../pr-review-pages";
import { s331Projects as projects } from "./projects";

export const searchBars: Scenario = {
  ...projects,
  name: "search-bars",
  says: "One search bar on System, Projects, Reviews and Issues",
  draft: { ...projects.draft, prReview: reviewSeed() },
};

// The review surface over the flight deck's ship, with the window left on the Cockpit: a pull request's link pasted there
// opens Reviews on it. The walk `paste-a-pr-link` plays it.

import type { Scenario } from "../moment";
import { s262PrReview } from "./pr-review";

export const s270PasteAPrLink: Scenario = {
  ...s262PrReview,
  name: "paste-a-pr-link",
  says: "A pull request's link pasted on the Cockpit opens its page in Reviews",
};

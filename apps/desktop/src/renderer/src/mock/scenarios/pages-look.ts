// Retros, Doctor and one Studio on one Fleet, so the walk `pages-look` can visit the three in the new look.

import { retroFixtures } from "@armada/jobs/fake";

import type { Scenario } from "../moment";
import { zoning } from "../studios-fake";
import { doctoring } from "../workflows-fleet";

const studied = zoning().scenario;
const { retros, lessons } = retroFixtures();

export const pagesLook: Scenario = {
  ...studied,
  name: "pages-look",
  says: "Two Jobs' retros, Fleet's probes with a warning and a failure, and one Studio",
  retros,
  lessons,
  state: { ...studied.state, health: doctoring().state.health },
};

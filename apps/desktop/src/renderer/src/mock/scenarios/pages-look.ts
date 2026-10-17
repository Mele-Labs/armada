// Retros, Doctor and one Studio on one Fleet, so the walk `pages-look` can visit the three in the new look.

import { slotsHeld } from "@armada/cleanup/fake";
import { retroFixtures } from "@armada/jobs/fake";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { zoning } from "../studios-fake";
import { doctoring } from "../workflows-fleet";

const studied = zoning().scenario;
const { fixtures, retros, lessons } = retroFixtures();

const pool = slotsHeld(fixtures[0]!.job, Date.now());
const board = holding("pages-look", "", fixtures, { alsoServed: [repository()] });

export const pagesLook: Scenario = {
  ...studied,
  reads: board.reads,
  name: "pages-look",
  says: "Two Jobs' retros, Fleet's probes with a warning and a failure, the slot pool and one Studio",
  retros,
  lessons,
  // The pool is read for the picked repository, so every slot is that repository's.
  held: { ...pool, slots: pool.slots?.map((one) => ({ ...one, manifest_id: repository().manifest!.id })) },
  state: { ...board.state, repository: studied.state.repository, health: doctoring().state.health },
};

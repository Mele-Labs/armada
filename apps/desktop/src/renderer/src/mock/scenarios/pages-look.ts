// Retros, Doctor and one Studio on one Fleet, so the walk `pages-look` can visit the three in the new look.

import { slotsHeld } from "@armada/cleanup/fake";
import { retroFixtures } from "@armada/screens/src/fixtures/retros";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { zoning } from "../studios-fake";
import { doctoring } from "../workflows-fleet";

const studied = zoning().scenario;
const { retros, lessons } = retroFixtures();

const pool = slotsHeld(Date.now());
const board = holding("pages-look", "");

export const pagesLook: Scenario = {
  ...studied,
  name: "pages-look",
  says: "Two Sessions' retros, Fleet's probes with a warning and a failure, the slot pool and one Studio",
  retros,
  lessons,
  // The pool is read for the picked repository, so every slot is that repository's.
  held: { ...pool, slots: pool.slots?.map((one) => ({ ...one, manifest_id: repository().manifest!.id })) },
  state: { ...board.state, repository: studied.state.repository, health: doctoring().state.health },
};

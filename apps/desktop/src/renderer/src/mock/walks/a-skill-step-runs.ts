// A Skill and a Drone step run (8 Oct 2026): `tidy` names a skill and fired when the PR opened, and a
// step added after `implement` carries a brief. Each runs on a Drone of its own, on a branch cut from
// the Job's, and the Job is not held. When a Drone commits, its branch asks where the change goes, as a
// repair's does: on this branch, or as a PR of its own. Every row is shaped as Fleet serves it.

import { button, inside, region, role, walk } from "../walk";

const RUN = region("This Job's run");

export const aSkillStepRuns = walk("real/job-2-side-runs", [
  { look: RUN, say: "A Job past PR open, on its canvas" },
  { look: inside(RUN, role("img", "Drone branch")), say: "A branch off the Skill Trigger and one off the added step" },
  { hover: inside(RUN, role("img", "Running")), say: "A Drone, running" },
  { look: inside(RUN, role("group", "Where the fix goes")), say: "A Drone committed: its branch holds and asks" },
  { look: inside(RUN, role("list", "The fix")), say: "What the change touches, beside the choice" },
  { press: inside(RUN, button("This branch")), say: "This branch" },
  { look: inside(RUN, role("img", "Passed")), say: "The step passes, its change on the Job's branch" },
  { press: inside(RUN, button("New PR")), say: "New PR" },
  { look: inside(RUN, role("img", "New PR")), say: "The Skill's branch ends in a PR of its own" },
  { look: inside(RUN, role("img", "Passed")), say: "And the Trigger passes" },
]);

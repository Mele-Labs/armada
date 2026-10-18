// Release, on the flight controls of an open Session: a confirmation that names each thing it holds, each a checkbox ticked when it
// opens, and asks where it goes, Release or another live Session of its repository. Cancel changes nothing; a hand moves what is ticked
// to the Session picked and the unticked stays; a release gives each back and says what stayed and why. Over `cockpit-release-holds`.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const tile = inside(controls, button("Release everything the Job holds"));
const asked = dialog(/^Release what /);
const pullRequest = inside(asked, role("checkbox", /#1861 Pin the store clock/));
const where = inside(asked, role("combobox", "Where it goes"));
const TAKER = "9e8f7a6b-5c4d-4e3f-9a2b-1c0d9e8f7a6b";

const cockpitReleaseHolds = walk("cockpit-release-holds", [
  { press: inside(list, button("Pin the store clock")), say: "A Job open on the stage, working, its own flight controls under it" },
  { look: tile, say: "Release is the sixth tile and the key 6" },
  { press: tile, say: "A confirmation opens over the deck" },
  { look: inside(asked, role("list", "What Pin the store clock holds")), say: "Each thing it holds by name, each ticked: two slots, its branch and its pull request" },
  { hover: inside(asked, role("img", "Pull request")), say: "Each mark names its kind on hover" },
  { look: inside(asked, button("Release all")), say: "Release is picked, and the confirm key says so. Cancel holds focus, so Enter cancels" },
  { type: TAKER, into: where, say: "The picker offers each live Job of the same repository" },
  { look: inside(asked, button("Hand to Review the store tests")), say: "The key names the Job it goes to, and the line under the list says each slot moves as it stands" },
  { press: inside(asked, button("Cancel")), say: "Cancel closes it and changes nothing" },
  { look: tile, say: "The tiles are as they were" },
  { press: tile, say: "Opened again" },
  { type: TAKER, into: where, say: "Review the store tests, picked" },
  { press: pullRequest, say: "The pull request unticked: it stays with this Job. Untick everything and the key turns off" },
  { press: inside(asked, button("Hand to Review the store tests")), say: "A slot is ticked, so the running turn is stopped, and Fleet hands over what is ticked" },
  { look: inside(controls, text("Handed to Review the store tests")), say: "What moved stands in place of the tiles: each slot and the branch" },
  { look: inside(controls, text("Not ticked")), say: "Still held: the pull request, which was not ticked" },
  { press: inside(controls, button("Close")), say: "Close brings the tiles back" },
  { hover: tile, say: "Still on: the Job holds its pull request" },
  { press: inside(list, button("Review the store tests")), say: "The Job it went to holds its own slot and branch and everything handed to it" },
  { press: tile, say: "Release on this one" },
  { look: inside(asked, button("Release all")), say: "Release, the default" },
  { press: inside(asked, button("Release all")), say: "Each slot's files are committed to its branch and the slot is freed" },
  { look: inside(controls, text("Committed crates/store/tests/clock.rs, crates/store/tests/pool.rs to fix/pin-store-clock")), say: "Released, with what was committed and where" },
  { look: inside(controls, text(/slot-6 is on no branch/)), say: "Still held, with why:slot-6's checkout is on no branch, so its files could not be committed" },
  { press: inside(controls, button("Close")), say: "Close brings the tiles back" },
]);

export { cockpitReleaseHolds as "cockpit-release-holds" };

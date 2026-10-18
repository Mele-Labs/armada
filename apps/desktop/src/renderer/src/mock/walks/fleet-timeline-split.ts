// The Cockpit's timeline, third take: split. The subjects in a column grouped by how they stand, and the feed beside
// them; a press on a subject narrows the feed to it. Over `fleet-timeline`.

import { button, inside, region, role, tab, walk } from "../walk";

const timeline = region("Timeline");
const subjects = role("navigation", "Subjects");

export const fleetTimelineSplit = walk("fleet-timeline", [
  { press: inside(region("Flight controls"), button("Timeline")), say: "Timeline, from the flight controls" },
  { press: tab("Split"), say: "Split: the subjects beside their events" },
  { look: timeline, say: "On the left every subject, grouped by how it stands, with how long since it moved. On the right the feed of all of them" },
  { hover: inside(subjects, button(/Job: Flaky store test/)), say: "A subject's mark names how it stands; only a subject that is working turns" },
  { press: inside(subjects, button(/Job: Debounce/)), say: "A press on a subject narrows the feed to it" },
  { look: timeline, say: "Its events alone, with the way to open it above them" },
  { press: inside(subjects, button(/Job: Debounce/)), say: "A second press lets the feed go" },
  { look: timeline, say: "All of them again" },
]);

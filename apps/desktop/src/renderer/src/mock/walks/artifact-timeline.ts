// A Job's Artifacts along time, in two takes of the same Job: a spine, with the Job's own line through the middle and each artifact
// branching off it where it appeared, and lanes, a row to each kind with the joins drawn across them. The walk opens the Job's Artifacts,
// reads the Issues and Links in the list, switches to the timeline, walks its points with the arrows, opens an old drawing of the sketch
// and goes back to the list. Over `artifact-timeline` and `artifact-timeline-lanes`, which draw one Job.

import { button, dialog, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { TIMELINE_SKETCH, TIMELINE_TITLE } from "../sessions/artifact-timeline";
import { kit } from "../sessions/walk-kit";

function steps(take: "spine" | "lanes"): Step[] {
  const { artifacts, close, key } = kit(false);
  const timeline = inside(artifacts, role("group", "Timeline", { exact: true }));
  const point = (name: RegExp) => inside(timeline, button(name));
  const sheet = dialog(`Sketch ${TIMELINE_SKETCH}`);
  return [
    { press: inside(region("Jobs", { exact: true }), button(TIMELINE_TITLE)), say: "A Job two hours in, with links, issues, a pull request, a sketch, a page and a file" },
    { press: key("Artifacts"), say: "Artifacts" },
    { look: inside(artifacts, region("Issues", { exact: true })), say: "The issues it works on, and below them the addresses you and the agent wrote in the thread" },
    { press: inside(artifacts, tab("Timeline")), say: "Timeline" },
    {
      look: timeline,
      say: take === "spine" ? "The whole Job along its own line, left to right in time. Each artifact branches off where it first appeared, with a point for each version" : "A lane for each kind, left to right in time. Each artifact is a line in its lane, with a point for each version",
    },
    { look: point(/^Checks failed, .*#1913/), say: "The pull request: opened, checks failed, checks passed, merged. The issue it closes is joined to it" },
    { key: "ArrowRight", on: point(/^Drawn, Store clock reads/), say: "The arrows walk the points, Enter opens one, and T switches back to the list" },
    { press: point(/^Drawn, Store clock reads/), say: "The first version of the sketch opens as it was drawn, not the latest" },
    { press: close(sheet), say: "Close" },
    { press: inside(artifacts, tab("List")), say: "Back to the list" },
  ];
}

const spine = walk("artifact-timeline", steps("spine"));
const lanes = walk("artifact-timeline-lanes", steps("lanes"));

export { spine as "artifact-timeline", lanes as "artifact-timeline-lanes" };

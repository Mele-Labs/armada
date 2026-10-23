// A Job's Artifacts along time, in two takes of the same Job: a spine, with the Job's own line through the middle and each artifact
// branching off it where it appeared, and lanes, a row to each kind with the joins drawn across them. The walk opens the Job's Artifacts,
// reads the Issues and Links in the list, switches to the timeline, walks its points with the arrows, opens an old drawing of the sketch
// and goes back to the list with T. Over `artifact-timeline` and `artifact-timeline-lanes`, which draw one Job.

import { button, dialog, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { TIMELINE_SKETCH, TIMELINE_TITLE } from "../sessions/artifact-timeline";
import { kit, toSessions } from "../sessions/walk-kit";

function steps(take: "spine" | "lanes"): Step[] {
  const { artifacts, close, key } = kit(false);
  const timeline = inside(artifacts, role("group", "Timeline", { exact: true }));
  const point = (name: RegExp) => inside(timeline, button(name));
  const sheet = dialog(`Sketch ${TIMELINE_SKETCH}`);
  return [
    toSessions,
    { press: inside(region("Jobs", { exact: true }), button(TIMELINE_TITLE)), say: "A Job two hours in, with links, issues, a pull request, a sketch, a page and a file" },
    { press: key("Artifacts"), say: "Artifacts" },
    { look: inside(artifacts, region("Issues", { exact: true })), say: "The issues it works on" },
    { look: inside(artifacts, region("Links", { exact: true })), say: "The addresses you and the agent wrote in the thread, by host and path" },
    { press: inside(artifacts, tab("Timeline")), say: "Timeline" },
    {
      look: timeline,
      say: take === "spine" ? "The Job along its own line, left to right in time. Each artifact branches off where it first appeared, with a point for each version" : "A lane for each kind, left to right in time. Each artifact is a line in its lane, with a point for each version",
    },
    { look: point(/^Checks failed, #1913/), say: "The pull request: opened, checks failed, checks passed, merged. The issue it closes is joined to it" },
    { key: "ArrowRight", on: point(/^Drawn, Store clock reads/), say: "Arrow keys walk the points: right and left along a line" },
    { key: "ArrowUp", on: point(/^Redrawn, Store clock reads/), say: "Up and down between lines. Enter opens a point, T switches back to the list" },
    { press: point(/^Drawn, Store clock reads/), say: "The first version of the sketch opens as it was drawn" },
    { look: sheet, say: "The first drawing, not the latest" },
    { press: close(sheet), say: "Close" },
    { key: "t", on: inside(artifacts, tab("Timeline")), say: "T" },
    { look: inside(artifacts, region("Sketches", { exact: true })), say: "Back in the list" },
  ];
}

const spine = walk("artifact-timeline", steps("spine"));
const lanes = walk("artifact-timeline-lanes", steps("lanes"));

export { spine as "artifact-timeline", lanes as "artifact-timeline-lanes" };

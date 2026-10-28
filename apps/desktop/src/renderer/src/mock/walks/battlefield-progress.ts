// A Session's work as a ring on the Cockpit battlefield, twelve hours in eight presses of `later`. One drone builds under its own
// shadow; what lies there is the `build` Fleet carries on the Session's record. #2301.

import { deckRow, inside, region, role, text, walk } from "../walk";

const map = role("listbox", "Map");
const places = role("navigation", "Places");
const dock = inside(map, role("option", "Rebuild the dock panel"));
const stage = region(/^Job: /);
// The workflow is at its second step, Implement, so Now is the current step's zone.
const now = region("Implement", { exact: true });
const tasks = inside(now, role("list", "Tasks"));
const subagents = inside(now, role("list", "Subagents"));
const opened = [
  { press: deckRow("Rebuild the dock panel"), say: "Open the Job" },
] as const;

const battlefieldProgress = walk("battlefield-progress", [
  { look: map, say: "Scope. One repository, five drones. The Job rebuilding the dock panel has read, been shown a walk and asked two questions, and edited nothing. A faint survey grid lies under its drone with a stake at each corner, and a small marker for the walk and for each question. Inside it a faint outline of a ring, whole and unsegmented: there is no task list yet, so nothing says how much is left. Outside it a thinner ring of faint arcs, the road this repository's Jobs usually take, with nothing lit" },
  { later: places, say: "It writes a list of seven tasks" },
  { look: map, say: "Plan. A ring lies on the ground under the drone, one dashed ghost segment for each task. The survey is gone" },
  { later: places, say: "Edits land, four subagents start, and four tasks are done" },
  { look: map, say: "Implement. A task done with edits seen is a solid segment. One done with none seen is outlined and hollow. The next task is open with edits seen: dashed, with an amber tick outside it. Four small drones stand still beside the ring while their subagents run" },
  { hover: dock, say: "Its card. The phase as a glyph and a word, and the plan as one bar with a segment for each task: solid, outlined, amber or faint" },
  ...opened,
  { look: tasks, say: "The Job open. Now lists its seven tasks, four done and three open, in the order it made them. A button with a speech bubble in its header asks the Job for an update" },
  { key: "Escape", on: stage, say: "Esc is back to the map" },
  { look: region("Bays"), say: "The bays are back" },
  { later: places, say: "Ten minutes with no tool call in a running turn. Three subagents end" },
  { look: map, say: "Stalled. An amber ring lies round the drone on the ground. The unfinished segments have dimmed. One small drone is still there: its subagent has not ended" },
  { later: places, say: "Thirty more minutes of quiet" },
  { look: map, say: "Forty minutes quiet. The unfinished segments are dimmer again" },
  { later: places, say: "It works again and finishes, shows a walk of the built thing, pushes and opens a pull request. Its turn ends" },
  { look: map, say: "The road. The walk, the push and the pull request are lit arcs. Checks and auto-merge are faint dashed arcs: this repository's Jobs usually do them next" },
  { hover: inside(map, role("option", /^Checks/)), say: "A faint arc says so, and a lit one says when" },
  { later: places, say: "Checks run and auto-merge is set" },
  { look: map, say: "Every stop is lit" },
  { later: places, say: "It merges" },
  { look: map, say: "Landed. The flag flashes and the last arc closes the circle. The ring goes after a moment" },
  { later: places, say: "Twenty-six minutes later" },
  { look: map, say: "The plot is bare. What landed is in Activity" },
  { press: deckRow("Rebuild the dock panel"), say: "Job page. Its row in the list carries one glyph for its phase, with a tooltip naming it" },
  { look: tasks, say: "Job page. Now lists the seven tasks, every one done. The road to main is the map's, not the page's" },
  { key: "Escape", on: stage, say: "Esc is back to the map" },
  { later: places, say: "A fourth Job, drawing subagents in the ring, writes a list. Its own agent wrote two tasks and two subagents wrote two each, one of them still running" },
  { look: map, say: "Subagents. The ring has six pieces. A piece a subagent wrote carries a dot at its middle. The running subagent is a small drone that stands by its two pieces and does not lap. The one that finished has no drone, and its pieces keep their dots" },
  { hover: inside(map, role("option", /^List the tooltips that name a worker/)), say: "A piece of a subagent says its task, where it stands and what the subagent was asked" },
  { hover: inside(map, role("option", /^Subagent, map the worker call sites/)), say: "The running worker says what it was asked" },
  { press: deckRow("Draw subagents in the ring"), say: "Open the Job" },
  { look: tasks, say: "Now lists the six tasks it wrote, in the order they were made: its own and the ones the two subagents wrote" },
  { look: inside(subagents, text("map the worker call sites")), say: "The running subagent is listed under them, with its type. The one that finished is not: Now lists only those at work" },
  { key: "Escape", on: stage, say: "Esc is back to the map" },
  { later: places, say: "A fifth Job splits the cockpit into panels with ten subagents. Their tasks reach the one list interleaved, round by round" },
  { press: deckRow("Split the cockpit panels"), say: "Open the Job" },
  { look: tasks, say: "Twenty-two tasks, the Job's own and ten subagents', in the one list" },
  { look: inside(subagents, text("split the deck panel")), say: "Six subagents are still running, each listed with its type. The four that finished are not" },
]);

export { battlefieldProgress as "battlefield-progress" };
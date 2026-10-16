// One view of a Session, on the Cockpit, which is the one place a Session is viewed: there is no Sessions page. Incoming calls stands on
// the left under the list and, with nothing selected, Activity on the right beside Bays. With a Session selected the right column holds
// Status, the Ledger and the Plan, the foot holds the Field and the Dispatch controls give way. The proposer's values stand where they
// already live: the name in the header, the model and effort in the composer, the workflow over the steps in Status. The Session goes
// from dispatched words to a pull request ready. Over the `one-session-view` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const stage = region(/^Session: /);
const status = region("Status");
const plan = region("Plan");
const ask = role("article", "Waiting on you");
const concern = (words: RegExp) => role("group", words);

export const oneSessionView = walk("one-session-view", [
  { look: places, say: "The top nav has no Sessions tab. A Session opens on the Cockpit from anywhere" },
  { look: region("Incoming calls"), say: "The Cockpit with no Session selected. Incoming calls stands on the left under the list, folded to its header while nothing is calling" },
  { look: region("Activity"), say: "Activity stands on the right, in the place Incoming calls had" },
  { look: region("Bays"), say: "Bays, above it" },
  { look: region("Autopilot"), say: "Autopilot, beside Flight control and Dispatch" },
  { drag: role("separator", "Resize Bays and Activity"), by: { x: 0, y: -120 }, say: "Every gap between panels is a handle, and the cursor is a row resize over it. Dragged, the panel above and the one below trade height, each to its minimum" },
  { key: "Home", on: role("separator", "Resize Bays and Activity"), say: "Home, or a double click on the gap, puts the two back to their own sizes" },
  { drag: role("separator", "Resize the left column"), by: { x: 60, y: 0 }, say: "A gap between columns is a handle too: the left column widens and the middle gives the width" },
  { key: "Home", on: role("separator", "Resize the left column"), say: "Back to its width. Sizes are kept for this layout and come back as left" },
  { press: deckRow("Retry the slot lease"), say: "A Session is selected. Its thread and composer fill the middle. Dispatch controls, Activity, Bays and Autopilot give way, and the Field is in the foot" },
  { look: role("combobox", "Model"), say: "Dispatched words and nothing run. The name is in the header, and the proposer has preset the model and the effort in the composer's own dropdowns, each marked as picked by it" },
  { look: status, say: "The proposed workflow is over the steps in Status, marked the same way. The steps run down a column with none lit yet" },
  { type: "Bug fix", into: role("combobox", "Workflow"), say: "A different workflow" },
  { look: text("Reproduce"), say: "Its steps replace the others" },
  { type: "none", into: role("combobox", "Workflow"), say: "None is a custom Session, which draws no steps" },
  { type: "Feature", into: role("combobox", "Workflow"), say: "Feature again" },
  { type: "medium", into: role("combobox", "Effort"), say: "Less effort, in the composer. The mark on it goes once he changes it" },
  { press: inside(status, button("Start")), say: "Start. The first step lights and the agent reads before it asks. The workflow is now a name and no longer a picker" },
  { look: inside(stage, ask), say: "Planning happens with him in the thread. The Status area has no plan yet, and the Field has the Session's own drone reading Fleet" },
  { press: inside(ask, role("radio", "Five")), say: "An answer to the agent's question" },
  { press: inside(ask, button("Answer")), say: "Answered" },
  { later: places, say: "The agent records its plan" },
  { look: inside(stage, role("article", "Plan to approve")), say: "The plan arrives as a card in the thread, drawn as a question is: each group with the type of subagent that takes it and its tasks, and Approve. He can answer in the composer instead to change it" },
  { look: plan, say: "The Plan panel on the right already shows the plan, with no approval band and no amber frame. The approval lives in the thread alone" },
  { press: inside(plan, role("button", /Lease retry in Fleet/)), say: "A group in the panel opens to its own paths and its tasks in full. Closed it is a row: its title, its subagent type and a glyph for each task" },
  { press: inside(plan, role("button", /Lease retry in Fleet/)), say: "And closes again" },
  { press: inside(stage, button("Approve")), say: "Approve, in the thread" },
  { look: inside(stage, role("article", "Plan approved")), say: "The card settles in the thread where the plan was recorded, as a record: the groups and a mark, no press left" },
  { look: region("Field"), say: "The build starts. Two subagents hover over the parts of the codebase they touch, the Session's own drone with them," },
  { look: status, say: "Build is lit in Status" },
  { look: plan, say: "Tasks tick in the plan as the agent reports them. The pencil on a group is edits Fleet saw" },
  { later: places, say: "The first two tasks finish and two more start" },
  { look: region("Field"), say: "The rust subagent stays, the protocol one is done and a bridge one has joined" },
  { look: region("Incoming calls"), say: "Nothing is calling, so Incoming calls is still folded to its header and the list has the room" },
  { later: places, say: "A judge reads the Build step" },
  { look: concern(/lease lock/), say: "A card for each concern, among the calls in Incoming calls. Nothing reaches the agent until he presses, and a judge never blocks the Session" },
  { look: region("Incoming calls"), say: "A concern is a call: Incoming calls has opened and the list has given up room, down to its minimum" },
  { key: "c", on: places, say: "C focuses Incoming calls. It opens further and lies over the list, and settles back when focus goes" },
  { press: inside(concern(/lease lock/), button("Send to session")), say: "Sent. It goes to the agent as his message and the card goes" },
  { press: inside(concern(/ceiling/), button("Dismiss")), say: "Dismissed. The agent is told nothing" },
  { later: places, say: "The agent finishes and runs its checks" },
  { look: plan, say: "Every task is done and the Plan is framed in green" },
  { look: status, say: "The steps are at Pull request" },
  { look: region("Ledger"), say: "The Ledger keeps its place and its size. The branch and the bay are two chips in its band, and the draft pull request is its row" },
  { drag: role("separator", "Resize Ledger and Plan"), by: { x: 0, y: -80 }, say: "With a Session selected the gaps in the right column and under the thread are handles too. The Ledger gives height to the Plan" },
  { key: "Home", on: role("separator", "Resize Ledger and Plan"), say: "Home puts them back" },
  { drag: role("separator", "Resize Thread and Foot"), by: { x: 0, y: -100 }, say: "The thread and the foot trade height, and the Field fills the foot" },
  { key: "Home", on: role("separator", "Resize Thread and Foot"), say: "Back to default. A folded panel has no handle beside it, and its header still opens it" },
  { press: inside(status, role("button", /Status/)), say: "A press on a header folds the panel to it. Every panel in the view does this, the Plan, Field, Ledger, Incoming calls, Jobs and Sessions and Flight control too" },
  { press: inside(status, role("button", /Status/)), say: "A second press opens it. Which panels are folded is kept for each one in the window's storage, so they come back as left" },
  { press: deckRow("Rotate the audit log"), say: "Another Session, on a workflow of twelve steps" },
  { look: status, say: "The steps scroll to the one it is at, and the ones far from it are out of sight. The plan is a panel of its own" },
]);

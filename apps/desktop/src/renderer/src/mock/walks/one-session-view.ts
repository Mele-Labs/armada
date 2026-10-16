// One view of a Session, the same on the Cockpit and on the Sessions page. With nothing selected the Cockpit keeps Activity, Bays and
// Autopilot, and the foot has Dispatch controls; with a Session selected the Dispatch controls go and the Session's own panels stand in
// Activity's and Autopilot's places, the Status area holds its workflow and plan in the ring's place, and the proposer's values stand
// where they already live: the name in the header, the model and effort in the composer, the workflow over the steps in Status. The Session goes from dispatched words to a pull request ready. Over the `one-session-view` scenario.
// Two takes, `&take=a` and `&take=b`, differ in where the judge's concerns stand and in how the Status area sits the plan by the steps.

import { toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const stage = region(/^Session: /);
const status = region("Status");
const ask = role("article", "Waiting on you");
const concern = (words: RegExp) => role("group", words);

export const oneSessionView = walk("one-session-view", [
  { look: region("Activity"), say: "The Cockpit with no Session selected. Activity, Bays, Dispatch controls and Autopilot are in their places" },
  { look: region("Bays"), say: "Bays" },
  { look: region("Autopilot"), say: "Autopilot, beside Flight control and Dispatch" },
  { press: deckRow("Retry the slot lease"), say: "A Session is selected. Its thread and composer fill the middle. Dispatch controls, Activity, Bays and Autopilot give way" },
  { look: role("combobox", "Model"), say: "Dispatched words and nothing run. The name is in the header, and the proposer has preset the model and the effort in the composer's own dropdowns, each marked as picked by it" },
  { look: status, say: "The proposed workflow is over the steps in Status, marked the same way. The steps run down a column with none lit yet, and the plan is empty" },
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
  { look: status, say: "The plan arrives for approval: the scope as paths and the task groups, each naming the type of subagent that takes it. The panel is framed in amber until he decides" },
  { press: inside(status, button("Approve")), say: "Approve. He could also answer in the thread to change the plan" },
  { look: region("Field"), say: "The build starts. Two subagents hover over the parts of the codebase they touch, the Session's own drone with them, in Activity's place" },
  { look: status, say: "Build is lit. Tasks tick as the agent reports them. The pencil on a group is edits Fleet saw" },
  { later: places, say: "The first two tasks finish and two more start" },
  { look: region("Field"), say: "The rust subagent stays, the protocol one is done and a bridge one has joined" },
  { look: region("Incoming calls"), say: "Nothing is calling, so Incoming calls is folded to its header and the Ledger has the room" },
  { later: places, say: "A judge reads the Build step" },
  { look: concern(/lease lock/), say: "A card for each concern. Take A puts them among Incoming calls and take B in Autopilot's place. Nothing reaches the agent until he presses, and a judge never blocks the Session" },
  { look: region("Ledger"), say: "In take A a concern is a call: Incoming calls has opened and the Ledger has given up room, down to its minimum" },
  { key: "c", on: places, say: "C focuses Incoming calls. It opens further and lies over the Ledger, and settles back when focus goes" },
  { press: inside(concern(/lease lock/), button("Send to session")), say: "Sent. It goes to the agent as his message and the card goes" },
  { press: inside(concern(/ceiling/), button("Dismiss")), say: "Dismissed. The agent is told nothing" },
  { later: places, say: "The agent finishes and runs its checks" },
  { look: status, say: "Every task is done and the steps are at Pull request. The panel is framed in green" },
  { look: region("Ledger"), say: "The Ledger keeps its place and its size. The branch and the bay are two chips in its band, and the draft pull request is its row" },
  { press: inside(status, role("button", /Status/)), say: "A press on a header folds the panel to it. Every panel in the view does this, the Field, Ledger, Incoming calls, Jobs and Sessions and Flight control too" },
  { press: inside(status, role("button", /Status/)), say: "A second press opens it. Which panels are folded is kept for each one in the window's storage, so they come back as left" },
  { press: deckRow("Rotate the audit log"), say: "Another Session, on a workflow of twelve steps" },
  { look: status, say: "The steps scroll to the one it is at, and the ones far from it are out of sight. The plan is under them in take A and under Build in take B" },
  toSessions,
  { press: inside(region("Sessions"), button(/Retry the slot lease/)), say: "The same Session on the Sessions page" },
  { look: status, say: "The same panels in the same places round the thread: the Field to the left, Status and Ledger to the right, and the Concerns under it" },
  { look: region(/^Session /), say: "There is no deck here, so the places are drawn round the thread. Incoming calls, Flight control and the Merge line are the Cockpit's own" },
]);

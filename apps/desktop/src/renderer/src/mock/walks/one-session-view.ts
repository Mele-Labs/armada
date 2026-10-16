// One view of a Session, the same on the Cockpit and on the Sessions page. With nothing selected the Cockpit keeps Activity, Bays and
// Autopilot; with a Session selected those three places hold the Session's own panels, and the Status area holds its workflow and
// plan in the ring's place. The Session goes from dispatched words to a pull request ready. Over the `one-session-view` scenario.
// Two takes, `&take=a` and `&take=b`, differ in which panel stands in which place and in how the Status area draws the plan.

import { toSessions } from "../sessions/walk-kit";
import { button, deckRow, inside, region, role, text, walk } from "../walk";

const places = role("navigation", "Places");
const stage = region(/^Session: /);
const status = region("Status");
const proposal = region("Proposal");
const ask = role("article", "Waiting on you");
const concern = (words: RegExp) => role("group", words);

export const oneSessionView = walk("one-session-view", [
  { look: region("Activity"), say: "The Cockpit with no Session selected. Activity, Bays and Autopilot are in their places" },
  { look: region("Bays"), say: "Bays" },
  { look: region("Autopilot"), say: "Autopilot, beside Flight control and Dispatch" },
  { press: deckRow("Retry the slot lease"), say: "A Session is selected. Its thread and composer fill the middle. Activity, Bays and Autopilot give way to the Session's panels" },
  { look: proposal, say: "Dispatched words and nothing run. The proposer has filled in the title, a workflow, the model and the effort, each one changeable before the work starts" },
  { look: status, say: "The Status area keeps its place. The workflow's steps are drawn with none lit yet, and the plan is empty" },
  { type: "Bug fix", into: role("combobox", "Workflow"), say: "A different workflow" },
  { look: text("Reproduce"), say: "Its steps replace the others" },
  { type: "none", into: role("combobox", "Workflow"), say: "None is a custom Session, which draws no steps" },
  { type: "Feature", into: role("combobox", "Workflow"), say: "Feature again" },
  { type: "sonnet", into: role("combobox", "Model"), say: "A lighter model" },
  { type: "medium", into: role("combobox", "Effort"), say: "And less effort" },
  { press: inside(proposal, button("Start")), say: "Start. The first step lights and the agent reads before it asks" },
  { look: inside(stage, ask), say: "Planning happens with him in the thread. The Status area has no plan yet, and the Field has the Session's own drone reading Fleet" },
  { press: inside(ask, role("radio", "Five")), say: "An answer to the agent's question" },
  { press: inside(ask, button("Answer")), say: "Answered" },
  { later: places, say: "The agent records its plan" },
  { look: status, say: "The plan arrives for approval: the scope as paths and the task groups, each naming the type of subagent that takes it. The panel is framed in amber until he decides" },
  { press: inside(status, button("Approve")), say: "Approve. He could also answer in the thread to change the plan" },
  { look: region("Field"), say: "The build starts. Two subagents hover over the parts of the codebase they touch, the Session's own drone with them" },
  { look: status, say: "Build is lit. Tasks tick as the agent reports them. The pencil on a group is edits Fleet saw" },
  { later: places, say: "The first two tasks finish and two more start" },
  { look: region("Field"), say: "The rust subagent stays, the protocol one is done and a bridge one has joined" },
  { later: places, say: "A judge reads the Build step" },
  { look: region("Concerns"), say: "A card for each concern. Nothing reaches the agent until he presses. A judge never blocks the Session" },
  { press: inside(concern(/lease lock/), button("Send to session")), say: "Sent. It goes to the agent as his message and the card goes" },
  { press: inside(concern(/ceiling/), button("Dismiss")), say: "Dismissed. The agent is told nothing" },
  { later: places, say: "The agent finishes and runs its checks" },
  { look: status, say: "Every task is done and the steps are at Pull request. The panel is framed in green" },
  { look: region("Ledger"), say: "The Ledger keeps its place. The branch and the bay are two chips in its band, and the draft pull request is its row" },
  toSessions,
  { press: inside(region("Sessions"), button(/Retry the slot lease/)), say: "The same Session on the Sessions page" },
  { look: status, say: "The same panels in the same places round the thread: Status and Ledger to the right, with the Field or Concerns where the take puts them" },
  { look: region(/^Session /), say: "There is no deck here, so the places are drawn round the thread. Incoming calls, Flight control and the Merge line are the Cockpit's own" },
]);

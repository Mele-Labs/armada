// What a Drone and a Judge write draws as markdown: a Drone's answers under
// its question, its turns, and the dock's questions from the Drone and the
// Judge. The stories and `Log.test.tsx` hold the claims; this is the look.

import { button, inside, role, tab, text, walk } from "../walk";

export const agentTextInMarkdown = walk("markdown/agent-text", [
  { look: text("move up"), say: "The Drone's answers draw their markdown: emphasis, a list, a name in code" },
  { press: tab("Drones"), say: "Every Drone that worked on the Job" },
  { press: button("Drone on T1"), say: "T1's Drone, opened on its turns" },
  { look: text("running_rows"), say: "Its closing turn is markdown: a list, and a name in code" },
  { press: button("Close"), say: "Back to the Job" },
  { press: button("Helm"), say: "The dock holds every question waiting on you" },
  {
    look: inside(role("article", "The drone asked a question"), text("hides it today")),
    say: "The Drone's question, in markdown",
  },
  { look: text("the order resets on the first save"), say: "The Judge's word on what the difference does, in markdown" },
]);

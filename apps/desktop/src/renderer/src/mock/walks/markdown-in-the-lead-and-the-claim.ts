// What a Drone writes draws as markdown where Armada quotes it: the question
// Overview's lead names, and the claim on the confidence sheet. A finding's
// View takes plain words for its title. Two walks, because the sites sit on
// two scenarios and a walk plays one.

import { button, dialog, inside, row, text, walk } from "../walk";

export const markdownInTheLead = walk("markdown/agent-text", [
  { look: text("A Drone asked you something"), say: "The lead names what is waiting on you" },
  {
    look: text("the plan's brief says draw it"),
    say: "The Drone's question under it: bold, a list, a name in code",
  },
]);

export const markdownInTheClaim = walk("job/review", [
  { press: button("What the Job captured"), say: "What the Job captured, at the gate" },
  { look: text("replaces the removed test"), say: "What the Drone says it did: bold, then a list of code" },
  { look: text("against a filled store"), say: "What it left alone: code and bold" },
  { press: button("For context"), say: "A finding with a View" },
  { press: inside(row("settings_store"), button("View")), say: "Its View" },
  {
    look: dialog("The consumers still import the old path, settings_store"),
    say: "The View's title: the finding's words, without its marks",
  },
]);

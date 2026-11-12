// The Operator is on: his dispatch goes to it before any Job exists. He stays where he was, and a toast names the Job it started or the
// live Job it handed his words to; a press opens it. Over `operator-dispatch`.

import { inside, region, role, walk } from "../walk";

const jobs = region("Jobs", {
  exact: true,
});
const frame = role("region", "Dispatch", {
  exact: true,
});
const request = inside(frame, role("textbox", "Request"));
const armada = inside(
  frame,
  role("radio", "Repository armada", {
    exact: true,
  }),
);
const start = inside(
  frame,
  role("button", "Start", {
    exact: true,
  }),
);
const message = role("textbox", "Message");

const operatorDispatch = walk("operator-dispatch", [
  {
    key: "Meta+n",
    on: jobs,
    say: "⌘N opens Dispatch",
  },
  {
    type: "Add a retry cap",
    into: request,
    say: "Words for a Job",
  },
  {
    press: armada,
    say: "The repository",
  },
  {
    press: start,
    say: "Start: the Operator takes the words first",
  },
  {
    look: role("button", /^Operator started/),
    say: "You stay on the list, and a toast names the Job it started",
  },
  {
    press: role("button", /^Operator started/),
    say: "A press opens it",
  },
  {
    look: message,
    say: "The Job, working on your words",
  },
  {
    key: "Meta+n",
    on: message,
    say: "Dispatch again",
  },
  {
    type: "Fix the migration order",
    into: request,
    say: "Words a live Job already covers",
  },
  {
    press: armada,
    say: "The repository",
  },
  {
    press: start,
    say: "Start",
  },
  {
    look: role("button", "Operator handed it to Store migration spike"),
    say: "The Operator handed them to the live Job; a press opens it",
  },
]);

export { operatorDispatch as "operator-dispatch" };

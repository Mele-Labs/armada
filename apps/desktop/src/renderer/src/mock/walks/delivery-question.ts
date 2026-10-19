// A bug or feature Job stops at its start and asks how its work is delivered. The question is a call in Needs you with three
// answers; the answer sends the choice, the call leaves, and the Job carries on. A first message that names a delivery starts
// the Job with no question. Over `delivery-question`.

import { button, inside, region, role, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const needs = region("Needs you");
const choice = (name: string) => inside(needs, role("radio", name, { exact: true }));

const deliveryQuestion = walk("delivery-question", [
  { key: "n", on: region("Jobs", { exact: true }), say: "Dispatch opens" },
  { type: "Fix the flaky ledger test", into: request, say: "A first message that does not say how to deliver" },
  { key: "Enter", on: request, say: "The Job starts" },
  { press: inside(needs, button(/^Needs you/)), say: "Its Needs you panel" },
  { look: inside(needs, text("How should this work be delivered?")), say: "The Job stops and asks" },
  { look: choice("Auto land"), say: "Auto land opens a pull request, watches ci and merges it" },
  { look: choice("Open PR only"), say: "Open PR only stops at the open pull request" },
  { look: choice("Keep local"), say: "Keep local stops at the commit, nothing pushed" },
  { key: "1", on: choice("Auto land"), say: "1 picks Auto land" },
  { press: inside(needs, button("Answer")), say: "Sent" },
  { look: role("textbox", "Message"), say: "The call has left the list and the Job carries on" },

  // A first message that names the delivery.
  { key: "n", on: region("Jobs", { exact: true }), say: "Dispatch opens again" },
  { type: "Fix the flaky clock test and open a PR", into: request, say: "A first message that says open a PR" },
  { key: "Enter", on: request, say: "The Job starts with no question" },
]);

export { deliveryQuestion as "delivery-question" };

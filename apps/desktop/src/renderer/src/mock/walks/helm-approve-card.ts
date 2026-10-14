// Helm asks the person to approve a Job. The dock draws the ask in the thread as a card: the Job it is
// for, Approve, and Not now. Approve is the press on the Job's approval gate, and the card reads from
// the Board that it was answered. A Job that moved on by another road reads that it is no longer
// waiting. Over `helm-approve-card`.

import { button, inside, region, role, text, walk } from "../walk";

const helm = role("complementary", "Helm");
const thread = inside(helm, region("Thread"));
const cards = inside(thread, region("Approval asked for"));

const helmApproveCard = walk("helm-approve-card", [
  { press: button("Helm"), say: "Helm's dock on armada" },
  { look: inside(thread, text("what is waiting on me?")), say: "Helm was asked what is waiting" },
  { look: inside(cards, text(/81-retire-the-legacy-poke-path/)), say: "Its ask to approve a Job is a card in the thread, with the Job's handle, workflow and steps" },
  { look: inside(cards, button("Approve")), say: "Approve, and Not now beside it" },
  { press: inside(cards, button("Approve")), say: "Approve is the Job's own gate press" },
  { look: inside(cards, text("Approved.")), say: "The card reads that it was answered" },
  { look: inside(thread, text("No longer awaiting approval.")), say: "The ask for a Job that moved on by another road reads that it is no longer waiting" },
]);

export { helmApproveCard as "helm-approve-card" };

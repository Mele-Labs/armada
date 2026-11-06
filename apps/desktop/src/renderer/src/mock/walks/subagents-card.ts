// While subagents run, one card at the thread's foot holds the one live mark: folded to how many run, opened to each one's
// status, task, last call and time. The rows above keep a still mark, and no Thinking line stands under them. Over the
// `thread-states` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const thread = region(/^Pane: Thread/);
const card = region("Subagents running");
const head = role("button", /subagents running$/);

const subagentsCard = walk("thread-states", [
  {
    press: deckRow("Theme audit"),
    say: "A Job that handed work to four subagents, one already back",
  },
  {
    look: inside(thread, card),
    say: "One card at the foot while three run, with the one live mark",
  },
  {
    press: head,
    say: "Opened, it lists the three running",
  },
  {
    look: inside(card, role("list")),
    say: "Each with its status, task, last call and time. A press opens its own thread",
  },
  {
    press: head,
    say: "Folded again",
  },
  {
    look: inside(thread, role("button", /^Subagent Audit the Cockpit for raw colours/)),
    say: "Its row in the thread: the status mark, the task, its time and its last call, quieter",
  },
]);

export { subagentsCard as "subagents-card" };

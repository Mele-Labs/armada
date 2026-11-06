// While subagents run, one card at the thread's foot holds the one live mark: folded to how many run, opened to each one's
// status, task, last call and time. In the thread a running one is a handoff line and a returned one its result, and no Thinking line stands under them. Over the
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
    say: "While one runs, its row in the thread is the handoff: one quiet line, the card being its live view",
  },
  {
    look: inside(thread, role("button", /^Subagent Audit the settings pages for raw colours/)),
    say: "Once one returns, its row is its result: how it ended and how long it took. A press opens its thread",
  },
]);

export { subagentsCard as "subagents-card" };

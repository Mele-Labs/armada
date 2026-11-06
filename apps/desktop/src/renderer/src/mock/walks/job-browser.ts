// The Job browser as a tab, used by the person and the agent at any time. The mark first in the toolbar is dim at rest and lit, with the act's
// sentence as its tooltip, while an agent act runs. The address takes an address and Enter whenever the person types one. Mock only: nothing is wired.
// Over the `job-browser-acts` scenario, whose `later` steps start an act and end it.

import { button, deckRow, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) =>
  region(`Pane: ${name}`, {
    exact: true,
  });
const empty = region("Empty pane", {
  exact: true,
});
const item = (name: string | RegExp) => role("menuitem", name);
const thread = inside(pane("Thread"), tab(/^Thread(: |$)/));
const browser = inside(
  pane("localhost:5173"),
  role("group", "Job browser", {
    exact: true,
  }),
);
const rest = inside(browser, role("img", "No agent act running"));
const acting = inside(browser, role("img", "Clicking Apply"));
const address = inside(
  browser,
  role("textbox", "Address", {
    exact: true,
  }),
);

const steps: Step[] = [
  toSessions,
  {
    press: deckRow("Retry the slot lease"),
    say: "A Job with a thread",
  },
  {
    key: "Meta+d",
    on: thread,
    say: "⌘D splits the pane",
  },
  {
    press: inside(empty, button("Add tab")),
    say: "+",
  },
  {
    press: item("Browser"),
    say: "Browser",
  },
  {
    hover: rest,
    say: "The agent's mark at rest. No agent act running",
  },
  {
    later: rest,
    say: "The agent starts an act",
  },
  {
    hover: acting,
    say: "The mark is lit. Clicking Apply",
  },
  {
    type: "http://localhost:5173/cart\n",
    into: address,
    say: "The address takes an address and Enter loads it while the agent acts",
  },
  {
    look: inside(browser, role("article", "Fernwood Supply cart")),
    say: "The cart",
  },
  {
    later: acting,
    say: "The act ends",
  },
  {
    look: rest,
    say: "The mark is dim again",
  },
];

const jobBrowser = walk("job-browser-acts", steps);

export { jobBrowser as "job-browser" };

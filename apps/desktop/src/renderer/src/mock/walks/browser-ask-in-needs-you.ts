// A page's ask in the Job browser is in the page and in the Job's Needs you, one ask drawn twice: the Job's row calls, the panel on the composer lists it, and the strip
// under the head asks it. Allow and Block in either settle it for that site and take both away. Mock only: nothing is wired. Over the `job-layouts` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
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
const thread = inside(
  pane("Thread"),
  tab("Thread", {
    exact: true,
  }),
);
const browser = inside(
  pane("localhost:5173"),
  role("group", "Job browser", {
    exact: true,
  }),
);
const page = inside(browser, role("article", "Fernwood Supply checkout"));
const lock = inside(
  browser,
  button("Site settings", {
    exact: true,
  }),
);
const settings = inside(browser, role("dialog", "Site settings"));
const strip = inside(browser, role("alertdialog", "Site request"));
const needs = region("Needs you", {
  exact: true,
});
const card = inside(needs, role("group", /^Allow .* for localhost:5173\?$/));

const steps: Step[] = [
  toSessions,
  {
    press: deckRow("Retry the slot lease"),
    say: "A Job with a Browser tab to open",
  },
  {
    key: "Meta+d",
    on: thread,
    say: "⌘D",
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
    press: inside(page, button("Connect Okta Verify")),
    say: "The page asks for the local network",
  },
  {
    look: inside(strip, text("It can reach apps on this Mac and devices on your network.")),
    say: "The ask is in the browser, with what Local network reaches",
  },
  {
    press: button("Needs you", {
      exact: true,
    }),
    say: "The same ask is in Needs you, on the composer",
  },
  {
    look: inside(card, text("It can reach apps on this Mac and devices on your network.")),
    say: "Allow says what Local network reaches",
  },
  {
    press: inside(
      card,
      role("radio", "Allow", {
        exact: true,
      }),
    ),
    say: "Allow",
  },
  {
    press: inside(
      card,
      button("Answer", {
        exact: true,
      }),
    ),
    say: "Answer",
  },
  {
    look: page,
    say: "The ask is gone from the page",
  },
  {
    press: lock,
    say: "Site settings",
  },
  {
    look: inside(
      inside(
        settings,
        role("radiogroup", "Local network", {
          exact: true,
        }),
      ),
      role("radio", "Allow", {
        exact: true,
      }),
    ),
    say: "Local network is Allow for this site",
  },
  {
    press: lock,
    say: "Close",
  },
  {
    press: inside(page, button("Use my location")),
    say: "The page asks for the location",
  },
  {
    look: inside(strip, text("localhost:5173 wants to use Location")),
    say: "Other asks carry no risk line",
  },
  {
    press: inside(
      strip,
      button("Block", {
        exact: true,
      }),
    ),
    say: "Block, in the browser",
  },
  {
    look: deckRow("Retry the slot lease"),
    say: "The item is gone from Needs you, and the row no longer calls",
  },
];

const browserAskInNeedsYou = walk("job-layouts", steps);

export { browserAskInNeedsYou as "browser-ask-in-needs-you" };

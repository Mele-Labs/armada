// Site settings in the Job browser: the lock before the address opens what the page may use, for this site and as the default for
// every site, and a page that asks raises a bubble under the lock. While the agent drives the choices are fixed and a page's ask is
// refused unseen. Mock only: nothing is wired. Over the `job-layouts` scenario, with the person taking the page over first.

import { button, deckRow, dialog, inside, region, role, tab, walk } from "../walk";
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
const settings = inside(browser, dialog("Site settings"));
const choose = (permission: string, choice: string) =>
  inside(
    inside(
      settings,
      role("radiogroup", permission, {
        exact: true,
      }),
    ),
    role("radio", choice, {
      exact: true,
    }),
  );
const view = (name: string) =>
  inside(
    inside(
      settings,
      role("radiogroup", "Settings for", {
        exact: true,
      }),
    ),
    role("radio", name, {
      exact: true,
    }),
  );
const okta = inside(page, button("Connect Okta Verify"));
const bubble = inside(browser, role("alertdialog", "Site request"));

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
    press: inside(browser, button(/^Agent driving/)),
    say: "Take the page over",
  },
  {
    look: browser,
    say: "The Job browser, with you driving",
  },
  {
    press: lock,
    say: "The lock before the address opens Site settings",
  },
  {
    look: inside(
      settings,
      role("radiogroup", "Local network", {
        exact: true,
      }),
    ),
    say: "Seven permissions, each Ask, Allow or Block",
  },
  {
    press: choose("Notifications", "Block"),
    say: "Notifications, Block, for this site",
  },
  {
    press: view("All sites"),
    say: "All sites edits the defaults",
  },
  {
    press: choose("Camera", "Block"),
    say: "Camera, Block, everywhere",
  },
  {
    press: lock,
    say: "Close",
  },
  {
    press: okta,
    say: "The page asks for the local network",
  },
  {
    look: bubble,
    say: "The site and what it wants, with Block and Allow",
  },
  {
    press: inside(
      bubble,
      button("Allow", {
        exact: true,
      }),
    ),
    say: "Allow",
  },
  {
    press: lock,
    say: "Reopen Site settings",
  },
  {
    look: choose("Local network", "Allow"),
    say: "Local network now shows Allow for this site",
  },
  {
    press: lock,
    say: "Close",
  },
  {
    press: inside(browser, button(/^You're driving/)),
    say: "Hand back to the agent",
  },
  {
    look: inside(browser, button("Take over to change site settings")),
    say: "The lock says why the choices are fixed",
  },
  {
    press: okta,
    say: "The page asks again",
  },
  {
    look: page,
    say: "No bubble appears while the agent drives",
  },
];

const sitePermissionsWalk = walk("job-layouts", steps);

export { sitePermissionsWalk as "site-permissions" };

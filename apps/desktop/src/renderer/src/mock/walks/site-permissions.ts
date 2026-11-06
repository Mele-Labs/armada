// Site settings in the Job browser: the lock before the address opens what the page may use, for this site and as the default for every site. Allow on Local network
// is confirmed first, with the risk said. Always open to the person; a page's ask is in Needs you (`browser-ask-in-needs-you`). Mock only: nothing is wired. Over the `job-layouts` scenario.

import { button, deckRow, dialog, inside, region, role, tab, walk } from "../walk";
import type { Step, Target } from "../walk";
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

const confirm = (title: string) => inside(browser, dialog(title));
const site = confirm("Allow local network for localhost:5173?");
const everywhere = confirm("Allow local network on all sites?");
const inDialog = (target: Target, name: string) =>
  inside(
    target,
    button(name, {
      exact: true,
    }),
  );

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
    press: lock,
    say: "The lock before the address opens Site settings",
  },
  {
    look: inside(
      settings,
      role("radiogroup", "Fullscreen", {
        exact: true,
      }),
    ),
    say: "Popups and Fullscreen follow Downloads. Every row works, whoever is on the page",
  },
  {
    press: choose("Local network", "Allow"),
    say: "Allow on Local network asks first",
  },
  {
    look: site,
    say: "The confirmation names the site and what it can reach",
  },
  {
    press: inDialog(site, "Allow"),
    say: "Allow. Enter does too, and Esc cancels",
  },
  {
    press: lock,
    say: "Reopen Site settings",
  },
  {
    press: view("All sites"),
    say: "All sites edits the defaults",
  },
  {
    press: choose("Local network", "Allow"),
    say: "Allow on Local network for every site",
  },
  {
    look: everywhere,
    say: "Its own confirmation",
  },
  {
    press: inDialog(everywhere, "Cancel"),
    say: "Cancel keeps the choice as it was",
  },
];

const sitePermissionsWalk = walk("job-layouts", steps);

export { sitePermissionsWalk as "site-permissions" };

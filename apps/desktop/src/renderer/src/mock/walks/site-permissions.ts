// Site settings in the Job browser: the lock before the address opens what the page may use, for this site and as the default for every site, and a page that asks
// raises a bubble under the lock. Allow on Local network is confirmed first, with the risk said. Always open to the person. Mock only: nothing is wired. Over the `job-layouts` scenario.

import { button, deckRow, dialog, inside, region, role, tab, text, walk } from "../walk";
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
    say: "Popups and Fullscreen follow Downloads",
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
    press: inDialog(site, "Cancel"),
    say: "Cancel",
  },
  {
    press: lock,
    say: "Local network is still Ask",
  },
  {
    press: choose("Local network", "Allow"),
    say: "Allow again",
  },
  {
    press: inDialog(site, "Allow"),
    say: "Allow confirms. Enter does too",
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
    say: "Cancel",
  },
  {
    press: lock,
    say: "Reopen Site settings",
  },
  {
    press: choose("Local network", "Ask"),
    say: "Back to Ask for this site, which needs no confirmation",
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
    look: inside(bubble, text("It can reach apps on this Mac and devices on your network.")),
    say: "The ask says what Local network reaches",
  },
  {
    press: inside(
      bubble,
      button("Allow", {
        exact: true,
      }),
    ),
    say: "Allow needs no second press",
  },
];

const sitePermissionsWalk = walk("job-layouts", steps);

export { sitePermissionsWalk as "site-permissions" };

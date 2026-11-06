// System › Browser: one set of defaults for every Job browser. A choice made there shows in the lock's All sites view, and Allow on Local network
// is confirmed in both. Mock only: nothing is wired. Over the `job-layouts` scenario.

import { button, deckRow, dialog, inside, region, role, tab, toSystemPage, walk } from "../walk";
import type { Step, Target } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) =>
  region(`Pane: ${name}`, {
    exact: true,
  });
const empty = region("Empty pane", {
  exact: true,
});
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
const group = (permission: string) =>
  role("radiogroup", permission, {
    exact: true,
  });
const radio = (permission: string, choice: string, within?: Target) =>
  inside(
    within === undefined ? group(permission) : inside(within, group(permission)),
    role("radio", choice, {
      exact: true,
    }),
  );
const everywhere = dialog("Allow local network on all sites?");

const steps: Step[] = [
  ...toSystemPage("Browser", "Browser, under Bridge"),
  {
    look: group("Fullscreen"),
    say: "Nine rows. Ask, Allow or Block for every Job browser",
  },
  {
    press: radio("Camera", "Block"),
    say: "Camera, Block",
  },
  {
    press: radio("Local network", "Allow"),
    say: "Local network, Allow",
  },
  {
    look: everywhere,
    say: "The confirmation says what Local network reaches",
  },
  {
    press: inside(
      everywhere,
      button("Allow", {
        exact: true,
      }),
    ),
    say: "Allow",
  },
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
    press: role("menuitem", "Browser"),
    say: "Browser",
  },
  {
    press: lock,
    say: "Site settings",
  },
  {
    press: inside(
      inside(
        settings,
        role("radiogroup", "Settings for", {
          exact: true,
        }),
      ),
      role("radio", "All sites", {
        exact: true,
      }),
    ),
    say: "All sites",
  },
  {
    look: radio("Camera", "Block", settings),
    say: "Camera is Block, as set in System",
  },
  {
    look: radio("Local network", "Allow", settings),
    say: "Local network is Allow",
  },
];

const browserDefaults = walk("job-layouts", steps);

export { browserDefaults as "browser-defaults" };

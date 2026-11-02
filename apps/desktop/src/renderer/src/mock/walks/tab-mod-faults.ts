// A tab mod switched off empties its tab, which says so, and switched on draws again. A tab mod Fleet refuses shows the reason and Fix, which starts a Mod Job on the tab
// that the tab follows until it draws again. Over the `job-layouts` scenario: a Custom tab built, then the Mod Job that built it, whose thread holds the mod's switch beside
// a pane showing the tab.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
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
const card = role("textbox", "Message the Mod Job");

const tabModFaults = walk("job-layouts", [
  toSessions,
  {
    press: deckRow("Retry the slot lease"),
    say: "A Job",
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
    press: item("Custom"),
    say: "Custom",
  },
  {
    type: "Pull requests waiting on me",
    into: inside(pane("Custom"), role("textbox", "Custom")),
    say: "Describe the tab",
  },
  {
    press: inside(pane("Custom"), button("Send")),
    say: "Send",
  },
  {
    later: inside(
      pane("Review queue"),
      tab("Review queue", {
        exact: true,
      }),
    ),
    say: "The Mod Job builds it",
  },
  {
    context: inside(
      pane("Review queue"),
      tab("Review queue", {
        exact: true,
      }),
    ),
    say: "The tab's menu",
  },
  {
    press: item("Open the Job that built it"),
    say: "The Mod Job that built it, which has no row in the list",
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
    press: item("Review queue"),
    say: "The tab, by name",
  },
  {
    look: inside(pane("Review queue"), button(/^Pull request #812/)),
    say: "The tab draws",
  },
  {
    press: role("switch", "Review-queue"),
    say: "The Job's thread holds the mod's switch: off",
  },
  {
    look: inside(pane("Review queue"), text("Switched off")),
    say: "The tab is empty and says why",
  },
  {
    press: role("switch", "Review-queue"),
    say: "On",
  },
  {
    look: inside(pane("Review queue"), button(/^Pull request #812/)),
    say: "It draws again",
  },
  {
    hover: inside(
      pane("Review queue"),
      tab("Review queue", {
        exact: true,
      }),
    ),
    say: "The tab's card",
  },
  {
    press: card,
    say: "Its box",
  },
  {
    type: "Import the forge client",
    into: card,
    say: "A change that breaks it",
  },
  {
    press: inside(role("tooltip"), button("Send")),
    say: "Send",
  },
  {
    key: "Escape",
    on: card,
    say: "Esc",
  },
  {
    look: inside(pane("Review queue"), text(/an `import` cannot be resolved/)),
    say: "Fleet refuses the mod: the tab shows why",
  },
  {
    press: inside(
      pane("Review queue"),
      button("Fix", {
        exact: true,
      }),
    ),
    say: "Fix starts a Mod Job on the tab, told the reason",
  },
  {
    look: inside(pane("Review queue"), role("img", /^Mod Fix Review queue/)),
    say: "The tab follows that Job while it works",
  },
  {
    later: inside(pane("Review queue"), button(/^Pull request #812/)),
    say: "It draws again",
  },
]);

export { tabModFaults };

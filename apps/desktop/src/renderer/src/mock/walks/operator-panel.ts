// The parts of the Operator panel the walk `operatorFull` did not show, before anything is wired (armada-ehhu.10). Over `operator-panel`.
// `?take=jobs-row` is the other take on where a raise with no Job pulses.

import { button, deckRow, inside, place, region, role, text, walk } from "../walk";
import type { Target } from "../walk";

// A dock in a wide window, a sheet in a narrow one.
const panel: Target = {
  ...role("complementary", "Operator", {
    exact: true,
  }),
  or: [
    role("dialog", "Operator", {
      exact: true,
    }),
  ],
};
const body = (name: string): Target =>
  inside(
    panel,
    role("tabpanel", name, {
      exact: true,
    }),
  );
const talk = body("Talk");
const permissions = body("Permissions");
const waiting = inside(body("Queue"), region("Waiting on you"));
const raise = (name: string): Target => inside(waiting, role("listitem", name));
const message = inside(talk, role("textbox", "Message to the Operator"));
const MAIN = "main is red: desktop_unit failed";
const REVIEW = "Pull request 412 asks for your review";
const LATER = "Would file: Record drops a step under a second";

export const operatorPanel = walk("operator-panel", [
  {
    look: {
      ...button("Operator", {
        exact: true,
      }),
      or: [
        button("Menu", {
          exact: true,
        }),
      ],
    },
    say: "Something was raised with no Job to stand on, so the Operator pulses. In a narrow window the button is in this menu",
  },
  {
    key: "Meta+j",
    on: button("Menu", {
      exact: true,
    }),
    say: "⌘J opens the panel, from anywhere",
  },
  {
    look: waiting,
    say: "The Queue is its own tab. Talk stays the discussion. What it waits on you for is listed here",
  },
  {
    look: raise(MAIN),
    say: "A red main, with its read: no Job is fixing it",
  },
  {
    look: raise(REVIEW),
    say: "A review request. Agent review stays on your press",
  },
  {
    hover: inside(raise(LATER), role("img", "What the Operator would do")),
    say: "File stands at Suggest here, so this row is what it would do",
  },
  {
    press: inside(
      raise(LATER),
      button("Do it"),
    ),
    say: "One press at the row's end does it. The pick is first, in a fixed column",
  },
  {
    press: inside(raise(MAIN), button("Dispatch")),
    say: "A raise takes one press too",
  },
  {
    press: {
      ...button("Repository", {
        exact: true,
      }),
      or: [
        button("Menu", {
          exact: true,
        }),
      ],
    },
    say: "The picker only narrows what the panel shows",
  },
  {
    press: {
      ...role("menuitem", "storefront"),
      or: [
        button("storefront"),
      ],
    },
    say: "Pick a repository",
  },
  {
    look: raise(REVIEW),
    say: "The Queue, the Log and Waiting to start show that repository's rows. The Operator still sees every Job on this Mac",
  },
  {
    press: {
      ...button("Repository", {
        exact: true,
      }),
      // A narrow window's menu stays open after a pick.
      or: [
        button(/^All/),
        button("Menu", {
          exact: true,
        }),
      ],
    },
    say: "Back to all of them",
  },
  {
    press: {
      ...role("menuitem", "All repositories"),
      or: [
        button(/^All/),
        permissions,
      ],
    },
    say: "All repositories",
  },
  {
    look: inside(permissions, button("Model: Sonnet")),
    say: "Permissions: the model, the same setting as Settings' system models",
  },
  {
    look: inside(permissions, text("Off in billing by standing order")),
    say: "A role says how a repository differs: this one by a standing order",
  },
  {
    look: inside(permissions, text("Suggest in storefront")),
    say: "This one by a setting",
  },
  {
    look: inside(permissions, role("list", "By repository")),
    say: "The settings are here, one column a role. Machine follows the row above",
  },
  {
    press: inside(permissions, button("Dispatch in storefront: Suggest")),
    say: "Each is a position",
  },
  {
    press: role("menuitem", "Machine: On"),
    say: "Machine drops the difference",
  },
  {
    look: inside(permissions, button("Add repository")),
    say: "Add repository draws another row",
  },
  {
    type: "Hold anything touching the Cockpit until the console Job ends\n",
    into: message,
    say: "Say an order in words",
  },
  {
    look: inside(talk, role("group", /^Standing order: Hold /)),
    say: "It comes back in the form Fleet enforces, and stands for nothing yet",
  },
  {
    press: inside(
      inside(talk, role("group", /^Standing order: Hold /)),
      button("Keep", {
        exact: true,
      }),
    ),
    say: "Keep is the only way it stands",
  },
  {
    look: inside(permissions, role("list", "Standing orders")),
    say: "It is in the list, removed in one press",
  },
  {
    key: "Meta+j",
    on: message,
    say: "⌘J shuts the panel",
  },
  {
    press: deckRow("Stop Silent Job Deaths"),
    say: "A Job the Operator has touched",
  },
  {
    look: text(/^The Operator started this Job/),
    say: "Its thread says when the Operator started it",
  },
  {
    look: text(/^The Operator sent:/),
    say: "Sent it words, in those exact words",
  },
  {
    look: text(/^The Operator answered "Keep the existing column"/),
    say: "Or answered for you",
  },
  {
    press: deckRow("Store Reports Its Size"),
    say: "A Job where Delegate stands at Suggest",
  },
  {
    press: inside(
      region("Needs you"),
      button("Needs you", {
        exact: true,
      }),
    ),
    say: "The same row in its own Needs you",
  },
  {
    look: inside(region("Needs you"), button(/^Do it/)),
    say: "With Do it and Skip at its end",
  },
  {
    press: place("Issues"),
    say: "Issues",
  },
  {
    press: button("Query", {
      exact: true,
    }),
    say: "The query menu",
  },
  {
    press: role("menuitem", "Reported"),
    say: "Reports are one kind here, and the band over the list is gone",
  },
  {
    press: button(/^A watch stayed live/),
    say: "A report reads as it did",
  },
  {
    press: inside(region("Reported bug"), button("Keep")),
    say: "Keep sets it aside",
  },
  {
    look: inside(region("Reported bug"), button("Dispatch")),
    say: "Its start act says Dispatch, as an issue's does, since the press may go to another Job or wait",
  },
]);

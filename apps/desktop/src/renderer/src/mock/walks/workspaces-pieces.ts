// The Workflows page over the lifecycles and rules Fleet keeps (30 Oct 2026): the lanes are read from the pieces a Fleet serves for store-web and its Workspace, a change is saved as the pieces
// it changes, and the person's own piece can be committed to the repository. A mock Fleet keeps the pieces for the window; the Workspace the long walk `workspaces-lanes` draws is the page's own mock.

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const group = (name: string) =>
  role("group", name, {
    exact: true,
  });
const LANES = region("Workflow", {
  exact: true,
});
const PANEL = role("dialog");
const STORE_WEB = group("store-web repository");
const kept = inside(PANEL, group("Where it is kept"));

// Played in a wide window, so each rule's label stands in the canvas beside the panel and a ring never lands on what the panel covers.
const pieces = walk(
  "workspaces-pieces",
  [
    ...toSystemPage("Workflows", "Workflows, from System"),
    {
      look: LANES,
      say: "The feature workflow over a lane each for Ticket, Pull request and Deploy, read from the lifecycles Fleet keeps",
    },
    {
      look: text("Edits here change every project and repository that keeps the default"),
      say: "Armada's feature is picked, so edits go to the default",
    },
    {
      press: inside(
        STORE_WEB,
        button("feature", {
          exact: true,
        }),
      ),
      say: "store-web's own copy: the line says edits here change store-web only, and its Ticket names In review as In QA",
    },
    {
      look: text("Edits here change store-web only"),
      say: "The picked row sets the level",
    },
    {
      look: inside(
        LANES,
        button("Ticket: In QA", {
          exact: true,
        }),
      ),
      say: "In QA is store-web's own name for the state, from the piece the repository committed",
    },
    {
      press: inside(
        LANES,
        button("Start the ticket", {
          exact: true,
        }),
      ),
      say: "A rule of yours: when the pull request enters Draft, move the ticket to In progress",
    },
    {
      look: inside(
        PANEL,
        role("combobox", "When", {
          exact: true,
        }),
      ),
      say: "When, If and Do are pickers over what Fleet can run, so what is chosen here is kept",
    },
    {
      look: inside(
        PANEL,
        role("combobox", "Add an act", {
          exact: true,
        }),
      ),
      say: "An act is added from the ones Fleet does, with its target and state",
    },
    {
      look: inside(PANEL, role("radio", "Acts")),
      say: "Autonomy is Acts",
    },
    {
      press: inside(PANEL, role("radio", "Asks")),
      say: "Choosing Asks on this tab saves store-web's own piece in your Kit, and nothing else",
    },
    {
      look: inside(
        PANEL,
        button("Use my-store's", {
          exact: true,
        }),
      ),
      say: "The tab now differs from the level above, and one press goes back to it",
    },
    {
      press: inside(
        kept,
        button("Commit to repository", {
          exact: true,
        }),
      ),
      say: "Commit to repository starts a Job of its own, with a pull request",
    },
    {
      look: inside(PANEL, text("Job started")),
      say: "The Job is started",
    },
    {
      press: inside(PANEL, button("Close")),
      say: "Closed",
    },
    {
      press: inside(
        LANES,
        button("Close the ticket", {
          exact: true,
        }),
      ),
      say: "A rule from store-web's repository",
    },
    {
      look: PANEL,
      say: "A rule store-web committed is the repository's: nothing to commit",
    },
    {
      press: inside(PANEL, button("Close")),
      say: "Closed",
    },
    {
      press: button("Add lifecycle", {
        exact: true,
      }),
      say: "A lifecycle is added on this tab, as a piece for store-web alone",
    },
    {
      look: inside(
        LANES,
        button("New lifecycle Tracker", {
          exact: true,
        }),
      ),
      say: "The new lane stands under Deploy",
    },
    {
      type: "Incident",
      into: inside(
        PANEL,
        role("textbox", "Name", {
          exact: true,
        }),
      ),
      say: "Named in the field, saved as it is left",
    },
    {
      press: inside(
        PANEL,
        button("Delete", {
          exact: true,
        }),
      ),
      say: "Delete takes its file out of your Kit",
    },
  ],
  {
    width: 2000,
    height: 1200,
  },
);

export { pieces as "workspaces-pieces" };

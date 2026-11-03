// A Jira ticket at Ready for QA, a status no Ticket state lists, taught from its row's menu: a right press on the ticket in Done lists the Ticket
// states with none ticked, In review teaches it, the row's glyph becomes In review's, and the Job's Workflow then has the ticket in In review. The
// same ticket's row in the Issues tab wears that glyph and has In review ticked, on the ContextMenu key. Over the `ticket-word` scenario. Mock only.

import { button, deckRow, inside, place, region, role, tab, text, walk } from "../walk";

const row = button(/^Issue ABC-12/);
const menu = role("menu");
const heading = inside(menu, text("ABC-12 is Ready for QA. In Armada that is"));
const state = (name: string) =>
  inside(
    menu,
    role("menuitem", name, {
      exact: true,
    }),
  );
const issues = region("Issues", {
  exact: true,
});
const lanes = role("group", "Workflow", {
  exact: true,
});

const ticketWord = walk("ticket-word", [
  {
    press: deckRow("Fix the doubled tax line"),
    say: "A Job holding ABC-12, which Jira has at Ready for QA",
  },
  {
    press: tab("Done", {
      exact: true,
    }),
    say: "Done, where the ticket stands",
  },
  {
    context: row,
    say: "A right press on the ticket",
  },
  {
    look: heading,
    say: "The menu names the ticket and its Jira status",
  },
  {
    look: state("In review"),
    say: "The four Ticket states, none ticked: no state lists Ready for QA",
  },
  {
    press: state("In review"),
    say: "In review",
  },
  {
    look: inside(
      row,
      role("img", "Ticket state: In review", {
        exact: true,
      }),
    ),
    say: "The ticket's glyph is In review's now, and its tooltip names the state",
  },
  {
    context: row,
    say: "The menu again",
  },
  {
    look: state("In review"),
    say: "In review is ticked now",
  },
  {
    key: "Escape",
    on: state("In review"),
    say: "Esc",
  },
  {
    press: place("Issues"),
    say: "The Issues tab",
  },
  {
    look: inside(
      issues,
      role("img", "Ticket state: In review", {
        exact: true,
      }),
    ),
    say: "Its row there has In review's glyph too",
  },
  {
    key: "ContextMenu",
    on: inside(issues, button(/^ABC-12 /)),
    say: "The same ticket's row, on the ContextMenu key",
  },
  {
    look: state("In review"),
    say: "The Ticket states over the View group, In review ticked",
  },
  {
    key: "Escape",
    on: state("In review"),
    say: "Esc",
  },
  {
    press: place("Cockpit"),
    say: "Back to the Job",
  },
  {
    press: inside(region(/^Pane: Thread/), button("Add tab")),
    say: "+",
  },
  {
    press: role("menuitem", "Workflow", {
      exact: true,
    }),
    say: "Its Workflow",
  },
  {
    look: inside(
      lanes,
      button("ABC-12: In review", {
        exact: true,
      }),
    ),
    say: "ABC-12 stands in In review, read from the word the Job's copy took",
  },
]);

export { ticketWord as "ticket-word" };

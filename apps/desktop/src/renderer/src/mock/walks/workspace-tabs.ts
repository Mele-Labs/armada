// A repository in two Workspaces (30 Oct 2026): the row picked in the list sets the level, so api's own copy edits api. api is in Platform and Mobile, which set
// Mark ready differently, so api runs the default's, the canvas warns, and the panel keeps either Workspace's as api's own. A mock, no Fleet behind it.

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const API_COPY = inside(
  role("group", "api repository", {
    exact: true,
  }),
  button("feature", {
    exact: true,
  }),
);
const LANES = region("Workflow", {
  exact: true,
});
const PANEL = role("dialog");

const tabs = walk(
  "workspace-tabs",
  [
    ...toSystemPage("Workflows", "Workflows, on api"),
    {
      press: API_COPY,
      say: "api's own copy of feature",
    },
    {
      look: text("Edits here change api only"),
      say: "The picked row sets the level: edits here change api only",
    },
    {
      look: inside(
        LANES,
        role("img", "Platform and Mobile differ", {
          exact: true,
        }),
      ),
      say: "Mark ready: Platform and Mobile differ",
    },
    {
      press: inside(
        LANES,
        button("Mark ready", {
          exact: true,
        }),
      ),
      say: "Mark ready",
    },
    {
      look: inside(
        PANEL,
        text("Platform and Mobile both change Mark ready. api runs the Default's."),
      ),
      say: "api runs the Default's",
    },
    {
      press: inside(
        PANEL,
        button("Use Mobile's", {
          exact: true,
        }),
      ),
      say: "Mobile's, kept as api's own",
    },
    {
      look: inside(
        PANEL,
        button("Use the default", {
          exact: true,
        }),
      ),
      say: "Mark ready is api's own, and the warning is gone",
    },
  ],
  {
    width: 2000,
    height: 1200,
  },
);

export { tabs as "workspace-tabs" };

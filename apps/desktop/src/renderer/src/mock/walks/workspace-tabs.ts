// A repository in two Workspaces (30 Oct 2026): the Runs in row has a group of tabs for the default, one for the Workspaces and one for the repositories. api is in Platform and Mobile, which set
// Mark ready differently, so api runs the default's, the canvas warns, and the panel keeps either Workspace's as api's own. A mock, no Fleet behind it.

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const RUNS_IN = role("group", "Runs in", { exact: true });
const LANES = region("Workflow", { exact: true });
const PANEL = role("dialog");

const tabs = walk(
  "workspace-tabs",
  [
    ...toSystemPage("Workflows", "Workflows, on api"),
    { look: inside(RUNS_IN, role("tab", "Mobile", { exact: true })), say: "Default, then Platform and Mobile, then api, web and ios" },
    { press: inside(RUNS_IN, role("tab", "api", { exact: true })), say: "api" },
    { look: inside(LANES, role("img", "Platform and Mobile differ", { exact: true })), say: "Mark ready: Platform and Mobile differ" },
    { press: inside(LANES, button("Mark ready", { exact: true })), say: "Mark ready" },
    { look: inside(PANEL, text("Platform and Mobile both change Mark ready. api runs the Default's.")), say: "api runs the Default's" },
    { press: inside(PANEL, button("Use Mobile's", { exact: true })), say: "Mobile's, kept as api's own" },
    { look: inside(PANEL, button("Use the default", { exact: true })), say: "Mark ready is api's own, and the warning is gone" },
  ],
  { width: 2000, height: 1200 },
);

export { tabs as "workspace-tabs" };

// A repository in two Workspaces (30 Oct 2026): the Runs in row has a tab for each Workspace, and a repository in both reads the default's where they set a rule differently. api is in Platform
// and Mobile. A mock, no Fleet behind it.

import { button, inside, region, role, toSystemPage, walk } from "../walk";

const RUNS_IN = role("group", "Runs in", { exact: true });
const tab = (name: string) => inside(RUNS_IN, role("tab", name, { exact: true }));
const LANES = region("Workflow", { exact: true });

const tabs = walk(
  "workspace-tabs",
  [
    ...toSystemPage("Workflows", "Workflows, on api"),
    { look: tab("All of Mobile"), say: "api is in Platform and in Mobile: All of Platform, All of Mobile, then api, web and ios" },
    { press: tab("All of Mobile"), say: "All of Mobile" },
    { press: tab("api"), say: "api" },
    { press: inside(LANES, button("Mark ready", { exact: true })), say: "Mark ready, as the default says" },
    { look: role("img", "Platform and Mobile differ", { exact: true }), say: "Platform and Mobile differ, so neither runs for api" },
  ],
  { width: 2000, height: 1200 },
);

export { tabs as "workspace-tabs" };

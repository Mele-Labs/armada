// Workflows on All for a repository that keeps no workflow copies (2 Nov 2026): lighthouse still has its group under Kept in, holding the workflows that run there.
// A row's switch in that group is lighthouse's, and Copy into lighthouse makes the repository's own copy and opens it. Over `workflows-empty-repository`.

import { button, inside, role, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const LIGHTHOUSE = group("lighthouse repository");
const HARBOR = group("harbor repository");

const workflowsEmptyRepository = walk("workflows-empty-repository", [
  ...toSystemPage("Workflows", "Workflows, from System, with all repositories"),
  { look: HARBOR, say: "Harbor keeps its own copies" },
  { look: LIGHTHOUSE, say: "Lighthouse keeps none. Its group lists the workflows that run there" },
  { press: inside(LIGHTHOUSE, role("switch", "On in lighthouse")), say: "A switch here turns the workflow off in lighthouse only" },
  { look: inside(LIGHTHOUSE, role("switch", "Off in lighthouse")), say: "Off in lighthouse" },
  { press: inside(LIGHTHOUSE, button("bug", { exact: true })), say: "Bug, as it runs in lighthouse" },
  { press: button("Copy into lighthouse", { exact: true }), say: "Copy into lighthouse" },
  { look: inside(LIGHTHOUSE, button("bug", { exact: true })), say: "Lighthouse now keeps its own bug, open to edit" },
]);

export { workflowsEmptyRepository as "workflows-empty-repository" };

// Workflows on All (2 Nov 2026): every repository has its group under Kept in, holding everything that runs there, its custom rows marked and in the accent. Harbor's copy of feature sits beside the
// workflows it keeps no copy of, and lighthouse, which keeps none, lists them all. A row's switch in that group is the repository's, and Copy into lighthouse
// makes the repository's own copy and opens it. Over `workflows-empty-repository`.

import { button, inside, role, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const LIGHTHOUSE = group("lighthouse repository");
const HARBOR = group("harbor repository");

const workflowsEmptyRepository = walk("workflows-empty-repository", [
  ...toSystemPage("Workflows", "Workflows, from System, with all repositories"),
  { look: inside(HARBOR, button("feature", { exact: true })), say: "Harbor keeps its own feature, its name in the accent" },
  { hover: inside(HARBOR, role("img", "harbor's own copy, replaces Armada's")), say: "Harbor's own copy, which replaces Armada's" },
  { look: inside(HARBOR, button("bug", { exact: true })), say: "Bug runs in harbor too, shipped with Armada: no mark, plain colour" },
  { hover: inside(HARBOR, role("img", "Yours")), say: "docs_only runs in harbor from Yours" },
  { look: LIGHTHOUSE, say: "Lighthouse keeps none. Its group lists the workflows that run there" },
  { press: inside(LIGHTHOUSE, role("switch", "On in lighthouse")), say: "A switch here turns the workflow off in lighthouse only" },
  { look: inside(LIGHTHOUSE, role("switch", "Off in lighthouse")), say: "Off in lighthouse" },
  { press: inside(LIGHTHOUSE, button("bug", { exact: true })), say: "Bug, as it runs in lighthouse" },
  { press: button("Copy into lighthouse", { exact: true }), say: "Copy into lighthouse" },
  { look: inside(LIGHTHOUSE, role("img", "lighthouse's own copy, replaces Armada's")), say: "Lighthouse keeps its own bug now, beside the rest that run there" },
]);

export { workflowsEmptyRepository as "workflows-empty-repository" };

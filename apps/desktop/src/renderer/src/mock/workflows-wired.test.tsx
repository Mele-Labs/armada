// The Workflows page, through `App` on a mock Fleet that serves the routes Fleet does: the list and its steps read from
// Fleet, a copy's words saved through `save_workflow` with the file's other fields as they were, and the changes an agent
// drafts read again while one is drafting. Fleet's own sentence is drawn as it said it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";
import { workflowing, workflowingOnAll } from "./workflows-fleet";

unmountAfterEach();

async function workflows(): Promise<ReturnType<typeof mount>> {
  const app = mount("workflows");
  await onScreen();
  await toSystemPage("Workflows");
  await expect.element(page.getByRole("heading", { name: "feature", exact: true })).toBeVisible();
  return app;
}

const group = (name: string) => page.getByRole("group", { name, exact: true });
const ARMADA = group("Armada");
const REPOSITORY = group("armada repository");
const YOURS = group("Yours");
const stepCard = (name: string) => group(name);
const words = (name: string) => stepCard(name).getByRole("textbox", { name: "Words" });
const wordsOf = (name: string) => () => (words(name).element() as HTMLTextAreaElement).value;

async function pickRepositoryFeature(): Promise<void> {
  await REPOSITORY.getByRole("button", { name: "feature", exact: true }).click();
  await expect.element(words("Walk")).toBeVisible();
}

/** The definitions saved, as they went to Fleet. */
function recordSaves(app: ReturnType<typeof mount>) {
  const saved: { scope: string; overwrite: boolean | undefined; definition: Record<string, unknown> }[] = [];
  const original = app.api.saveWorkflow;
  app.api.saveWorkflow = async (saving) => {
    saved.push({ scope: saving.body.scope, overwrite: saving.body.overwrite, definition: JSON.parse(saving.body.definition) as Record<string, unknown> });
    return original(saving);
  };
  return saved;
}

test("the list is Fleet's, grouped as Armada, the repository and Yours, and Armada's feature is open read-only", async () => {
  await workflows();
  for (const name of ["bug", "design_plan", "feature", "prototype"]) {
    await expect.element(ARMADA.getByRole("button", { name, exact: true })).toBeVisible();
  }
  await expect.element(REPOSITORY.getByRole("button", { name: "feature", exact: true })).toBeVisible();
  await expect.element(YOURS.getByRole("button", { name: "docs_only", exact: true })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "This repository's copy replaces Armada's" })).toBeVisible();
  await expect.element(page.getByText("adding or changing something a person uses")).toBeVisible();
  expect(page.getByRole("textbox", { name: "Words" }).elements()).toHaveLength(0);
});

test("a copy's steps say what ends each and when one is passed over, from Fleet", async () => {
  await workflows();
  await pickRepositoryFeature();
  await expect.element(stepCard("Walk")).toHaveTextContent("you approve the walk");
  await expect.element(stepCard("Walk")).toHaveTextContent("nothing would be seen");
  await expect.element(stepCard("Deliver")).toHaveTextContent("it is merged");
});

test("saving a step's words sends the file with that step's instructions changed and every other field as it was", async () => {
  const app = await workflows();
  const saved = recordSaves(app);
  await pickRepositoryFeature();
  const before = JSON.parse((await app.api.readWorkflowDefinition("feature", "repository", null).then((one) => (one.ok ? one.definition.definition : "{}")))) as {
    steps: { id: string }[];
  };

  await userEvent.fill(words("Propose"), "Say it in one line.");
  await stepCard("Propose").getByRole("button", { name: "Save", exact: true }).click();
  await expect.poll(() => saved.length).toBe(1);

  expect(saved[0]!.scope).toBe("repository");
  expect(saved[0]!.overwrite).toBe(true);
  const sent = saved[0]!.definition as { steps: { id: string; instructions?: string }[] };
  expect(sent.steps.find((one) => one.id === "propose")?.instructions).toBe("Say it in one line.");
  // Everything but that one step's words is the file as it was.
  const restored = { ...sent, steps: sent.steps.map((one) => (one.id === "propose" ? { ...one, instructions: (before.steps.find((each) => each.id === "propose") as { instructions?: string }).instructions } : one)) };
  expect(restored).toEqual(before);
  await expect.element(stepCard("Propose").getByRole("button", { name: "Save", exact: true })).not.toBeInTheDocument();
  await expect.element(words("Propose")).toHaveValue("Say it in one line.");
});

test("a save Fleet refuses is drawn as Fleet said it and the words stay", async () => {
  const app = await workflows();
  app.api.saveWorkflow = async () => ({
    ok: false,
    outcome: { ok: false, why: "refused", error: { code: "fleet.workflow_unfit", message: "Names a check this repository does not declare", run_id: "mock", fields: {}, chain: [] } },
  });
  await pickRepositoryFeature();
  await userEvent.fill(words("Propose"), "Changed.");
  await stepCard("Propose").getByRole("button", { name: "Save", exact: true }).click();
  await expect.element(page.getByRole("alert")).toHaveTextContent("Names a check this repository does not declare");
  await expect.element(words("Propose")).toHaveValue("Changed.");
});

test("Compare with Armada's reads Armada's words of the same id and marks the step that differs", async () => {
  await workflows();
  await pickRepositoryFeature();
  await page.getByRole("button", { name: "Compare with Armada's" }).click();
  await expect.element(stepCard("Refine").getByRole("group", { name: "Compared with Armada's" })).toBeVisible();
  await expect.element(stepCard("Refine").getByRole("img", { name: "Differs from Armada's" })).toBeVisible();
  expect(stepCard("Propose").getByRole("group", { name: "Compared with Armada's" }).elements()).toHaveLength(0);
});

test("Reset to Armada's writes Armada's words back and the step no longer differs", async () => {
  await workflows();
  await pickRepositoryFeature();
  await expect.poll(wordsOf("Refine")).toMatch(/Run the repository's checks before pushing/);
  await page.getByRole("button", { name: "Reset to Armada's" }).click();
  await expect.element(page.getByText("Matches Armada's")).toBeVisible();
  await expect.element(words("Refine")).toHaveValue("Wire it to the real thing and write the tests now. Show a new walk only if the look changed.");
});

test("Delete removes the copy, and the list opens what runs in its place", async () => {
  await workflows();
  await REPOSITORY.getByRole("button", { name: "feature", exact: true }).click();
  await page.getByRole("button", { name: "Delete" }).click();
  await expect.element(REPOSITORY).not.toBeInTheDocument();
  await expect.element(ARMADA.getByRole("button", { name: "feature", exact: true })).toHaveAttribute("aria-current", "true");
});

test("Make my copy reads Armada's definition, saves it to Kit without overwrite, and opens the copy under Yours", async () => {
  const app = await workflows();
  const saved = recordSaves(app);
  await ARMADA.getByRole("button", { name: "bug", exact: true }).click();
  await page.getByRole("button", { name: "Make my copy" }).click();
  await expect.element(YOURS.getByRole("button", { name: "bug", exact: true })).toHaveAttribute("aria-current", "true");
  expect(saved).toHaveLength(1);
  expect(saved[0]).toMatchObject({ scope: "kit", overwrite: false });
  expect(saved[0]!.definition).toMatchObject({ workflow_id: "bug" });
  await expect.element(words("Find the cause")).toBeVisible();
});

test("Make my copy of one Kit already holds opens the copy that is there", async () => {
  await workflows();
  await ARMADA.getByRole("button", { name: "bug", exact: true }).click();
  await page.getByRole("button", { name: "Make my copy" }).click();
  await expect.element(YOURS.getByRole("button", { name: "bug", exact: true })).toHaveAttribute("aria-current", "true");
  await ARMADA.getByRole("button", { name: "bug", exact: true }).click();
  await page.getByRole("button", { name: "Make my copy" }).click();
  await expect.element(YOURS.getByRole("button", { name: "bug", exact: true })).toHaveAttribute("aria-current", "true");
  expect(page.getByRole("alert").elements()).toHaveLength(0);
});

test("Share is the acts on a piece of the person's own", async () => {
  await workflows();
  await YOURS.getByRole("button", { name: "docs_only", exact: true }).click();
  await expect.element(page.getByRole("button", { name: "Commit to repository" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Share", exact: true })).toBeVisible();
});

test("Propose to Armada shows the Job it started and opens it in the Cockpit", async () => {
  await workflows();
  const opened: string[] = [];
  window.addEventListener("armada:open-session-in-cockpit", (event) => opened.push((event as CustomEvent<string>).detail));
  await page.getByRole("button", { name: "Propose to Armada" }).click();
  await expect.element(page.getByText("Sent. A Job started on the Armada repository")).toBeVisible();
  await page.getByRole("status").getByRole("button", { name: "Open Job" }).click();
  expect(opened).toEqual(["mock-armada-proposal"]);
});

test("a Job Fleet nudged opens in the Cockpit, and Not a drift takes the row and the suggestion drawn from it", async () => {
  await workflows();
  await pickRepositoryFeature();
  const opened: string[] = [];
  window.addEventListener("armada:open-session-in-cockpit", (event) => opened.push((event as CustomEvent<string>).detail));
  const rail = stepCard("Walk").getByRole("group", { name: /^Cockpit timeline rail/ });
  await rail.getByRole("button", { name: "Open Job" }).click();
  expect(opened).toEqual(["timeline-rail"]);

  await expect.element(stepCard("Approve plan").getByRole("group", { name: /^Pause and resume/ })).toBeVisible();
  await stepCard("Approve plan").getByRole("button", { name: "Suggest a change" }).click();
  await expect.element(stepCard("Approve plan").getByRole("group", { name: "Suggested change" })).toBeVisible();
  await stepCard("Approve plan").getByRole("button", { name: "Not a drift" }).click();
  await expect.element(stepCard("Approve plan").getByRole("group", { name: /^Pause and resume/ })).not.toBeInTheDocument();
  await expect.element(stepCard("Approve plan").getByRole("group", { name: "Suggested change" })).not.toBeInTheDocument();
});

test("N on a drift row marks it not a drift", async () => {
  await workflows();
  await pickRepositoryFeature();
  const row = stepCard("Walk").getByRole("group", { name: /^Pause and resume/ });
  row.element().focus();
  await userEvent.keyboard("n");
  await expect.element(row).not.toBeInTheDocument();
  await expect.element(stepCard("Walk").getByRole("group", { name: /^Cockpit timeline rail/ })).toBeVisible();
});

test("a suggested change drawn from two Jobs is approved into the step's words", async () => {
  const app = await workflows();
  const saved = recordSaves(app);
  await pickRepositoryFeature();
  const card = stepCard("Walk").getByRole("group", { name: "Suggested change" });
  await expect.element(card).toBeVisible();
  await card.getByRole("button", { name: "Approve", exact: true }).click();
  await expect.element(card).not.toBeInTheDocument();
  await expect.poll(wordsOf("Walk")).toMatch(/Test files wait for the approval too/);
  // The approval is Fleet's own save, and not one Bridge made.
  expect(saved).toHaveLength(0);
});

test("a suggested change is dismissed with Esc and the step keeps its words", async () => {
  await workflows();
  await pickRepositoryFeature();
  const card = stepCard("Walk").getByRole("group", { name: "Suggested change" });
  await expect.element(card).toBeVisible();
  card.element().focus();
  await userEvent.keyboard("{Escape}");
  await expect.element(card).not.toBeInTheDocument();
  expect(wordsOf("Walk")()).not.toMatch(/Test files wait/);
});

async function ask(text: string): Promise<void> {
  const box = page.getByRole("textbox", { name: "Tell an agent what to change" });
  await userEvent.fill(box, text);
  box.element().focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
}

test("a change typed to an agent shows it working, then the draft, and Approve saves it", async () => {
  await workflows();
  await pickRepositoryFeature();
  await ask("split Refine into Wire and Tests");
  await expect.element(page.getByRole("img", { name: "The agent is working" }).first()).toBeVisible();
  const region = page.getByRole("region", { name: "Proposed change" });
  await expect.element(region).toBeVisible();
  await expect.element(region.getByRole("group", { name: "Wire", exact: true })).toBeVisible();
  await region.getByRole("button", { name: "Approve", exact: true }).click();
  await expect.element(stepCard("Wire")).toBeVisible();
  await expect.element(stepCard("Tests")).toBeVisible();
  await expect.element(stepCard("Wire")).toHaveTextContent("it is wired to the real thing");
  expect(stepCard("Refine").elements()).toHaveLength(0);
});

test("a draft sent back with notes is drafted again, and a dismissed one leaves the workflow as it was", async () => {
  const app = await workflows();
  await pickRepositoryFeature();
  const sent: string[] = [];
  const original = app.api.sendBackWorkflowChange;
  app.api.sendBackWorkflowChange = async (back) => {
    sent.push(back.notes);
    return original(back);
  };
  await ask("split Refine into Wire and Tests");
  const region = page.getByRole("region", { name: "Proposed change" });
  await region.getByRole("button", { name: "Send back" }).click();
  await userEvent.fill(region.getByRole("textbox", { name: "Notes for the agent" }), "Keep Refine's checks line");
  await region.getByRole("button", { name: "Send notes" }).click();
  await expect.poll(() => sent).toEqual(["Keep Refine's checks line"]);
  await expect.element(page.getByRole("img", { name: "The agent is working" }).first()).toBeVisible();
  await expect.element(region).toBeVisible();

  await region.getByRole("button", { name: "Dismiss" }).click();
  await expect.element(page.getByRole("textbox", { name: "Tell an agent what to change" })).toBeVisible();
  await expect.element(stepCard("Refine")).toBeVisible();
});

test("a request that changes nothing is said with Fleet's reason", async () => {
  await workflows();
  await pickRepositoryFeature();
  await ask("make it nicer");
  await expect.element(page.getByRole("alert")).toHaveTextContent("The drafted workflow is the same as the one it was made from");
});

test("an Approve refused because the file changed is drawn as Fleet said it", async () => {
  const app = await workflows();
  await pickRepositoryFeature();
  await ask("split Refine into Wire and Tests");
  const region = page.getByRole("region", { name: "Proposed change" });
  await expect.element(region).toBeVisible();
  // The file changes under the draft.
  const held = await app.api.readWorkflowDefinition("feature", "repository", null);
  if (!held.ok) throw new Error("no feature");
  const changed = JSON.stringify({ ...JSON.parse(held.definition.definition), name: "renamed" });
  await app.api.saveWorkflow({ manifestId: app.scenario.state.holds.manifests[0]?.id ?? null, body: { scope: "repository", definition: changed, overwrite: true } });
  await region.getByRole("button", { name: "Approve", exact: true }).click();
  await expect.element(region.getByRole("alert")).toHaveTextContent(/changed since the draft was made/);
  await expect.element(region).toBeVisible();
});

test("a file Fleet left out shows with its reason", async () => {
  const app = mount(workflowing());
  const original = app.api.readWorkflows;
  app.api.readWorkflows = async () => {
    const read = await original();
    return read.ok
      ? { ...read, leftOut: [{ id: "hotfix", source: "kit", file: "~/.armada/workflows/hotfix.json", said: "Names the check smoke, which this repository does not declare" }] }
      : read;
  };
  await onScreen();
  await toSystemPage("Workflows");
  await YOURS.getByRole("button", { name: "hotfix", exact: true }).click();
  await expect.element(page.getByRole("img", { name: "Left out" }).first()).toBeVisible();
  await expect.element(page.getByRole("paragraph").filter({ hasText: "Left out" })).toHaveTextContent("Names the check smoke, which this repository does not declare");
});

test("↑ and ↓ move through the list and Enter goes into the steps", async () => {
  await workflows();
  await ARMADA.getByRole("button", { name: "bug", exact: true }).click();
  await ARMADA.getByRole("button", { name: "bug", exact: true }).element().focus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.poll(() => document.activeElement?.textContent).toBe("design_plan");
  await userEvent.keyboard("{ArrowUp}");
  await expect.poll(() => document.activeElement?.textContent).toBe("bug");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => document.activeElement?.closest(".armada-wm__detail") !== null).toBe(true);
});

test("F reaches the find box and narrows the list", async () => {
  await workflows();
  (document.activeElement as HTMLElement | null)?.blur();
  await userEvent.keyboard("f");
  await expect.poll(() => (document.activeElement as HTMLInputElement | null)?.type).toBe("search");
  await userEvent.keyboard("docs");
  await expect.element(YOURS.getByRole("button", { name: "docs_only", exact: true })).toBeVisible();
  expect(ARMADA.elements()).toHaveLength(0);
});

test("on All repositories each repository's copies sit in a group of its own, and no Manifest is asked for", async () => {
  mount(workflowingOnAll());
  await onScreen();
  await toSystemPage("Workflows");
  await expect.element(ARMADA).toBeVisible();
  await expect.element(group("armada repository")).toBeVisible();
  await expect.element(group("ledger repository")).toBeVisible();
  await expect.element(YOURS).toBeVisible();
  await expect.element(group("ledger repository").getByRole("button", { name: "bug", exact: true })).toBeVisible();
  expect(page.getByText("has no Manifest yet").elements()).toHaveLength(0);
  expect(ARMADA.getByRole("button", { name: "bug", exact: true }).elements()).toHaveLength(1);
});

test("on All repositories a copy's words save under its own repository and are kept when read again", async () => {
  const app = mount(workflowingOnAll());
  const saved = recordSaves(app);
  await onScreen();
  await toSystemPage("Workflows");
  const ledger = group("ledger repository");
  await ledger.getByRole("button", { name: "bug", exact: true }).click();
  await expect.element(words("Fix")).toBeVisible();
  expect(wordsOf("Fix")()).toContain("Add a line to the changelog.");
  const edited = "Make the smallest change that addresses the cause. Write no tests yet.";
  await words("Fix").fill(edited);
  await stepCard("Fix").getByRole("button", { name: "Save", exact: true }).click();
  await expect.poll(() => saved.length).toBe(1);
  // Read again: leave the copy for Armada's, come back to ledger's.
  await ARMADA.getByRole("button", { name: "bug", exact: true }).click();
  await ledger.getByRole("button", { name: "bug", exact: true }).click();
  await expect.element(words("Fix")).toBeVisible();
  expect(wordsOf("Fix")()).toBe(edited);
  // The first repository's copies were not touched.
  expect(group("armada repository").getByRole("button", { name: "bug", exact: true }).elements()).toHaveLength(0);
});

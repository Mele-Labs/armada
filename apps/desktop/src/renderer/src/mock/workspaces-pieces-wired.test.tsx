// The Workflows page's lanes over the lifecycles and rules a Fleet keeps, through `App`: an edit is saved as the piece it changes and read back, a refusal is drawn in Fleet's words and the edit goes
// back to what Fleet holds, a Fleet without the save routes leaves the editor read only and says nothing, and a rule changed on Armada's workflow makes no copy of the workflow.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";
import { workspacesPieces } from "./scenarios/workspaces-pieces";

unmountAfterEach();

async function lanes(): Promise<ReturnType<typeof mount>> {
  const app = mount(workspacesPieces);
  await onScreen();
  await toSystemPage("Workflows");
  await expect.element(page.getByRole("region", { name: "Workflow", exact: true })).toBeVisible();
  return app;
}

const PANEL = page.getByRole("dialog");
const radio = (name: string) => PANEL.getByRole("radio", { name, exact: true });

async function openProgress(): Promise<void> {
  await page.getByRole("region", { name: "Workflow", exact: true }).getByRole("button", { name: "Start the ticket", exact: true }).click();
  await expect.element(radio("Acts")).toBeChecked();
}

test("an edit to a rule is saved as the person's own piece and read back from Fleet", async () => {
  const app = await lanes();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  const copied = vi.spyOn(app.api, "saveWorkflow");
  await openProgress();
  await radio("Asks").click({ force: true });
  await expect.element(radio("Asks")).toBeChecked();
  expect(saved).toHaveBeenCalledTimes(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({ overwrite: true, piece: { kind: "rule", rule: { id: "progress", autonomy: "asks", scope: { kind: "everywhere" } } } });
  // The rules are pieces: a change to one is no change to Armada's workflow, which would make a copy of it.
  expect(copied).not.toHaveBeenCalled();
  const read = await app.api.listLifecycleRules("01M1CNPKTV0018H2M1CXDNBK06");
  expect(read.ok && read.value.rules.find((one) => one.rule.id === "progress")?.rule.autonomy).toBe("asks");
});

test("a piece Fleet refuses is said in its words and the edit goes back to what Fleet holds", async () => {
  const app = await lanes();
  const error = { code: "fleet.piece_unfit", message: "progress moves a ticket to a state ticket does not have", run_id: "r", fields: {}, chain: [] };
  app.api.saveLifecyclePiece = async () => ({ ok: false, outcome: { ok: false, why: "refused", error } });
  await openProgress();
  await radio("Asks").click({ force: true });
  await expect.element(page.getByText("progress moves a ticket to a state ticket does not have")).toBeVisible();
  await expect.element(radio("Acts")).toBeChecked();
});

test("a Fleet that does not serve the save routes leaves the editor read only, with nothing said", async () => {
  const app = await lanes();
  app.api.saveLifecyclePiece = async () => ({ ok: false, outcome: { ok: false, why: "transport", detail: "404", fault: { method: "POST", path: "/lifecycles/pieces/save", why: "unanswerable", status: 404 } } });
  await openProgress();
  await radio("Asks").click({ force: true });
  await expect.element(radio("Acts")).toBeChecked();
  await expect.element(radio("Asks")).toBeDisabled();
  expect(page.getByRole("alert").elements()).toHaveLength(0);
});

test("a rule or a state is not told to an agent, which drafts changes to steps, and a step is", async () => {
  await lanes();
  const region = page.getByRole("region", { name: "Workflow", exact: true });
  await region.getByRole("button", { name: "Start the ticket", exact: true }).click();
  await expect.element(page.getByRole("toolbar", { name: "Edit rule Start the ticket" })).toBeVisible();
  expect(page.getByRole("button", { name: "Tell an agent rule Start the ticket" }).elements()).toHaveLength(0);
  await PANEL.getByRole("button", { name: "Close" }).click();
  await region.getByRole("button", { name: "Step: Build", exact: true }).click();
  await expect.element(page.getByRole("button", { name: "Tell an agent step Build" })).toBeVisible();
});

test("a rule the person keeps offers Commit to repository, and one the repository committed does not", async () => {
  // Wide enough that the rule the repository committed stands in the canvas once the panel is closed, which puts the canvas back where it was.
  await page.viewport(1800, 900);
  await lanes();
  const region = page.getByRole("region", { name: "Workflow", exact: true });
  await region.getByRole("button", { name: "Start the ticket", exact: true }).click();
  await expect.element(PANEL.getByRole("group", { name: "Where it is kept" }).getByRole("button", { name: "Commit to repository" })).toBeVisible();
  await PANEL.getByRole("button", { name: "Close" }).click();
  await region.getByRole("button", { name: "Close the ticket", exact: true }).click();
  await expect.element(PANEL.getByRole("radio", { name: "Asks", exact: true })).toBeChecked();
  expect(PANEL.getByRole("button", { name: "Commit to repository" }).elements()).toHaveLength(0);
});

const region = () => page.getByRole("region", { name: "Workflow", exact: true });
const pick = (name: string) => PANEL.getByRole("combobox", { name, exact: true });

test("the pickers of a rule change its piece, and an act that needs words is added once they are typed", async () => {
  const app = await lanes();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await openProgress();
  await PANEL.getByRole("combobox", { name: "When" }).selectOptions("ends|build");
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({ overwrite: true, piece: { rule: { id: "progress", when: { kind: "step_ends", step: "build" } } } });
  // Choosing the act saves nothing until it has its message.
  await pick("Add an act").selectOptions("tell");
  expect(saved).toHaveBeenCalledTimes(1);
  await PANEL.getByRole("textbox", { name: "Message" }).fill("Start on it");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => saved.mock.calls.length).toBe(2);
  expect(saved.mock.calls[1]![0]).toMatchObject({ piece: { rule: { does: [{ what: { act: "tracker_move" } }, { what: { act: "tell", message: "Start on it" } }] } } });
});

test("a lifecycle is added on the tab, named, and deleted, each kept as its own piece", async () => {
  const app = await lanes();
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  const deleted = vi.spyOn(app.api, "deleteLifecyclePiece");
  await page.getByRole("button", { name: "Add lifecycle", exact: true }).click();
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({ piece: { kind: "lifecycle", lifecycle: { id: "new_lifecycle", name: "New lifecycle", scope: { kind: "everywhere" } } } });
  const name = PANEL.getByRole("textbox", { name: "Name" });
  await name.fill("Incident");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => saved.mock.calls.length).toBe(2);
  expect(saved.mock.calls[1]![0]).toMatchObject({ overwrite: true, piece: { lifecycle: { id: "new_lifecycle", name: "Incident" } } });
  await PANEL.getByRole("button", { name: "Delete", exact: true }).click();
  await expect.poll(() => deleted.mock.calls.length).toBe(1);
  expect(deleted.mock.calls[0]![0]).toMatchObject({ kind: "lifecycle", id: "new_lifecycle", scope: { kind: "everywhere" } });
});

test("a lifecycle added stands in the frame, panel open or closed, since the frame grows with its lanes", async () => {
  await lanes();
  await page.getByRole("button", { name: "Add lifecycle", exact: true }).click();
  const head = region().getByRole("button", { name: "New lifecycle Tracker", exact: true });
  const inside = () => {
    const frame = region().getByRole("group", { name: "feature workflow" }).element().getBoundingClientRect();
    const box = head.element().getBoundingClientRect();
    return box.top >= frame.top && box.bottom <= frame.bottom;
  };
  await expect.element(head).toBeVisible();
  expect(inside()).toBe(true);
  await PANEL.getByRole("button", { name: "Close" }).click();
  await expect.poll(inside).toBe(true);
});

test("what Fleet cannot run is not offered: no add on a forge lifecycle, no delete on Armada's, no step edits below the default", async () => {
  await lanes();
  await region().getByRole("button", { name: "Pull request: Draft", exact: true }).click();
  await expect.element(page.getByRole("toolbar", { name: "Edit pull request Draft" })).toBeVisible();
  expect(page.getByRole("button", { name: "Add after pull request Draft" }).elements()).toHaveLength(0);
  await PANEL.getByRole("button", { name: "Close" }).click();
  await region().getByRole("button", { name: "Ticket", exact: false }).first().click();
  expect(PANEL.getByRole("button", { name: "Delete", exact: true }).elements()).toHaveLength(0);
  await PANEL.getByRole("button", { name: "Close" }).click();
  await page.getByRole("tab", { name: "store-web", exact: true }).click();
  await region().getByRole("button", { name: "Step: Build", exact: true }).click();
  expect(page.getByRole("button", { name: "Add after step Build" }).elements()).toHaveLength(0);
  expect(PANEL.getByRole("textbox", { name: "Name" }).elements()).toHaveLength(0);
  expect(PANEL.getByRole("combobox", { name: "Ends when" }).elements()).toHaveLength(0);
});

test("a rule the repository committed that is deleted stays, marked with when it goes", async () => {
  await page.viewport(1800, 900);
  const app = await lanes();
  const committed = vi.spyOn(app.api, "commitToRepository");
  await region().getByRole("button", { name: "Close the ticket", exact: true }).click();
  await page.getByRole("button", { name: "Delete rule Close the ticket" }).click();
  await expect.poll(() => committed.mock.calls.length).toBe(1);
  expect(committed.mock.calls[0]![0]).toMatchObject({ remove: true, piece: { id: "close-on-merge" } });
  await region().getByRole("button", { name: "Close the ticket", exact: true }).hover();
  await expect.element(page.getByText("Removed from the repository when its pull request merges")).toBeVisible();
});

test("a Start a Job rule offers the repository's workflows, keeps the one it names when its prompt is changed, and keeps a new pick", async () => {
  const seed = workspacesPieces.draft!.pieces!;
  const kick = {
    rule: {
      id: "kick",
      name: "Kick it off",
      scope: { kind: "everywhere" as const },
      when: { kind: "enters" as const, lifecycle: "ticket", state: "in_review" },
      conditions: [{ kind: "at_step" as const, step: "build" }],
      does: [{ kind: "start_job", autonomy: "acts" as const, what: { act: "start_job" as const, workflow: "feature", prompt: "Look at it" } }],
      on_failure: { retries: 2, then: "ask_owner" as const },
      autonomy: "acts" as const,
    },
    kept: { rank: { reach: "everywhere" as const, source: "kit" as const }, path: "/Users/user/.armada/rules/kick.toml" },
  };
  const app = mount({ ...workspacesPieces, draft: { pieces: { ...seed, rules: [...seed.rules, kick] } } });
  await onScreen();
  await toSystemPage("Workflows");
  const saved = vi.spyOn(app.api, "saveLifecyclePiece");
  await region().getByRole("button", { name: "Kick it off", exact: true }).click();
  const workflow = PANEL.getByRole("combobox", { name: "Start a Job 1 workflow", exact: true });
  await expect.element(workflow).toHaveValue("feature");
  await PANEL.getByRole("textbox", { name: "Prompt" }).fill("Look again");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => saved.mock.calls.length).toBe(1);
  expect(saved.mock.calls[0]![0]).toMatchObject({ piece: { rule: { id: "kick", does: [{ what: { act: "start_job", workflow: "feature", prompt: "Look again" } }] } } });
  await workflow.selectOptions("planning");
  await expect.poll(() => saved.mock.calls.length).toBe(2);
  expect(saved.mock.calls[1]![0]).toMatchObject({ piece: { rule: { id: "kick", does: [{ what: { act: "start_job", workflow: "planning", prompt: "Look again" } }] } } });
});

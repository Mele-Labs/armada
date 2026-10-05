// The Workflow creator, through `App` on a mock Fleet that serves the same
// routes Fleet does: a definition read as text, a save checked by Fleet, and
// Fleet's own sentence drawn as it said it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { workflowing } from "./workflows-fleet";

unmountAfterEach();

async function creator(): Promise<ReturnType<typeof mount>> {
  const app = mount("workflows");
  await onScreen();
  await page.getByRole("button", { name: "Workflows", exact: true }).click();
  return app;
}

const frame = () => page.getByRole("region", { name: "Workflow definition" });
const save = () => frame().getByRole("button", { name: "Save", exact: true });

async function renameTo(id: string): Promise<void> {
  await frame().getByRole("button", { name: "Settings", exact: true }).click();
  await userEvent.fill(page.getByRole("textbox", { name: "Workflow id" }), id);
  await page.getByRole("button", { name: "Close" }).click();
}

test("a save over an id that is already there shows Fleet's sentence, and a second press replaces it", async () => {
  await creator();
  await page.getByRole("button", { name: "migration, repository" }).click();
  await expect.element(page.getByRole("button", { name: /^Schema/ })).toBeVisible();
  await renameTo("bug");

  await save().click();
  await expect.element(page.getByText("Not saved")).toBeVisible();
  await expect.element(page.getByText(/already defines `bug` and nothing was written/)).toBeVisible();

  await save().click();
  await expect.element(page.getByText(/^Saved$/)).toBeVisible();
  await expect.element(page.getByText("Not saved")).not.toBeInTheDocument();
});

test("a name Fleet will not file under is refused in Fleet's words", async () => {
  await creator();
  await page.getByRole("button", { name: "migration, repository" }).click();
  await renameTo("not a name");
  await save().click();
  await expect.element(page.getByText(/cannot be a file's name/)).toBeVisible();
});

test("an edited workflow goes out as Fleet's shape: no structure key, and a back edge only through verdict_routing", async () => {
  const { api } = await creator();
  const sent: string[] = [];
  const original = api.saveWorkflow;
  api.saveWorkflow = async (saving) => {
    sent.push(saving.body.definition);
    return original(saving);
  };
  await page.getByRole("button", { name: "bug, repository" }).click();
  await expect.element(page.getByRole("button", { name: /^Review/ })).toBeVisible();
  await save().click();
  await expect.element(page.getByText(/^Saved$/)).toBeVisible();

  const body = JSON.parse(sent[0]!) as { workflow_id: string; structure?: unknown; steps: { id: string; verdict_routing?: unknown; iteration_cap?: number }[] };
  expect(body.workflow_id).toBe("bug");
  expect(body.structure).toBeUndefined();
  expect(body.steps.find((one) => one.id === "review")).toMatchObject({ verdict_routing: { request_changes: "fix" }, iteration_cap: 4 });
  expect(body.steps.find((one) => one.id === "repro")).not.toHaveProperty("iteration_cap");
});

test("Discuss with Helm hands over the draft's text, naming the workflows screen", async () => {
  const { api } = await creator();
  const asked: { text: string; screen: string | undefined }[] = [];
  api.askHelm = async (text, context) => {
    asked.push({ text, screen: context?.screen });
    return { ok: true };
  };
  await page.getByRole("button", { name: "release_notes, kit" }).click();
  await page.getByRole("button", { name: "Discuss with Helm" }).click();
  await expect.element(page.getByRole("img", { name: "Handed to Helm" })).toBeVisible();
  expect(asked).toHaveLength(1);
  expect(asked[0]!.screen).toBe("workflows");
  expect(JSON.parse(asked[0]!.text)).toMatchObject({ workflow_id: "release_notes" });
});

test("the rail warns only while Fleet says a file was left out, and draws no count", async () => {
  const scenario = workflowing();
  mount(scenario);
  await onScreen();
  await expect.element(page.getByRole("img", { name: "A workflow file cannot run" })).toBeVisible();
});

test("a Fleet that does not say a file was left out draws no mark", async () => {
  const scenario = workflowing();
  mount({
    ...scenario,
    state: { ...scenario.state, health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "acting" } } },
  });
  await onScreen();
  await expect.element(page.getByRole("button", { name: "Workflows", exact: true })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "A workflow file cannot run" })).not.toBeInTheDocument();
});

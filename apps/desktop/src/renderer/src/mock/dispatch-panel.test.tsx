// The dispatch panel through `App` on the mock Fleet's `dispatch-panel-from-either-bar` scenario, three
// set-up repositories with armada busiest: the chip's repository as Fleet reads it, asked inline when
// it cannot tell, the three kinds, the strip a send leaves, and the draft. The look is the two walks'
// (`dispatch-panel-from-either-bar`, `dispatched-strip-opens-it`); this holds what they decided.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SCENARIO = "dispatch-panel-from-either-bar";
const NAMING_BRIDGE = "Show the Drone's branch on each Bridge cockpit tile";
const NAMING_NOTHING = "Clean up the old retry logic";
const ARMADA = repository().root;
const STOREFRONT = "/Users/user/code/storefront";

const frame = () => page.getByRole("region", { name: "Dispatch", exact: true });
const request = () => page.getByRole("textbox", { name: "Request" });
const chip = (name: string) => frame().getByRole("button", { name });
const act = (name: string) => frame().getByRole("button", { name, exact: true });
const strip = () => page.getByRole("group", { name: "Last dispatch" });
const choices = () => frame().getByRole("listbox", { name: "Repository" });

async function cockpit() {
  const app = mount(SCENARIO);
  await onScreen();
  return app;
}

test("the words are read for their repository, and a request naming none is left unresolved", async () => {
  await cockpit();
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository, finding")).toBeVisible();
  await expect.element(chip("Repository armada")).toBeVisible();

  await request().fill(NAMING_NOTHING);
  await expect.element(chip("Repository, not found")).toBeVisible();
  // Nothing asks until Dispatch is pressed.
  expect(frame().element().hasAttribute("data-asking")).toBe(false);
});

test("a repository read from the words is the one the Job is sent to", async () => {
  const app = await cockpit();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  expect(propose).toHaveBeenCalledWith(NAMING_BRIDGE, [], ARMADA);
});

test("a request that names none asks inline, and the pick sends it", async () => {
  const app = await cockpit();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_NOTHING);
  await expect.element(chip("Repository, not found")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(choices()).toBeVisible();
  expect(propose).not.toHaveBeenCalled();
  // Fleet ranks armada first: it has the Jobs. Every set-up repository is listed.
  const names = [...choices().element().querySelectorAll('[role="option"] .armada-dispatch-panel__name')].map((one) => one.textContent);
  expect(names).toEqual(["armada", "billing", "storefront"]);
  await choices().getByRole("option", { name: /storefront/ }).click();
  await expect.element(strip()).toBeVisible();
  expect(propose).toHaveBeenCalledWith(NAMING_NOTHING, [], STOREFRONT);
});

test("an answer to words that have since changed is let go", async () => {
  const app = await cockpit();
  const answers: ((read: { manifest_id: string | null; likely: string[] }) => void)[] = [];
  const asked = vi.spyOn(app.api, "readRequestRepository").mockImplementation(
    () => new Promise((resume) => answers.push(resume)),
  );
  await request().fill(NAMING_BRIDGE);
  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(1));
  await request().fill(NAMING_NOTHING);
  await vi.waitFor(() => expect(asked).toHaveBeenCalledTimes(2));

  // The second answers first, with no pick; the first, for the old words, answers late with one.
  answers[1]!({ manifest_id: null, likely: [] });
  await expect.element(chip("Repository, not found")).toBeVisible();
  answers[0]!({ manifest_id: repository().manifest!.id, likely: [repository().manifest!.id] });
  await new Promise((resume) => setTimeout(resume, 50));
  await expect.element(chip("Repository, not found")).toBeVisible();
  expect(chip("Repository armada").query()).toBeNull();
});

test("a repository picked by hand beats the one read from the words", async () => {
  const app = await cockpit();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository armada")).toBeVisible();
  await chip("Repository armada").click();
  await choices().getByRole("option", { name: /storefront/ }).click();
  await expect.element(chip("Repository storefront")).toBeVisible();
  // Words read again do not take the pick back.
  await request().fill(`${NAMING_BRIDGE} again`);
  await new Promise((resume) => setTimeout(resume, 800));
  await expect.element(chip("Repository storefront")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  expect(propose).toHaveBeenCalledWith(`${NAMING_BRIDGE} again`, [], STOREFRONT);
});

// The owner, 13 Oct 2026: on All, a Session asks where it starts, as a Job does.
test("a Session on All that names no repository asks inline, and the pick starts it there", async () => {
  const app = await cockpit();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_NOTHING);
  await userEvent.keyboard("{Tab}");
  await expect.element(frame().getByRole("tab", { name: "Session" })).toHaveAttribute("aria-selected", "true");
  await expect.element(chip("Repository, not found")).toBeVisible();
  await act("Start").click();
  await expect.element(choices()).toBeVisible();
  await choices().getByRole("option", { name: /storefront/ }).click();
  await expect.element(page.getByRole("textbox", { name: "Message" })).toBeVisible();
  expect(propose).not.toHaveBeenCalled();
});

test("a Session whose words name a repository starts there without asking", async () => {
  await cockpit();
  await request().fill(NAMING_BRIDGE);
  await userEvent.keyboard("{Tab}");
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Start").click();
  await expect.element(page.getByRole("textbox", { name: "Message" })).toBeVisible();
  expect(page.getByRole("listbox", { name: "Repository" }).query()).toBeNull();
});

test("an Autopilot is read first, and started from what was read", async () => {
  const app = await cockpit();
  const read = vi.spyOn(app.api, "proposeAutopilot");
  const start = vi.spyOn(app.api, "startAutopilot");
  await request().fill("Work through the open Bridge bugs until 7am");
  await frame().getByRole("tab", { name: "Autopilot" }).click();
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Read it").click();
  await expect.element(page.getByRole("textbox", { name: "Instruction" })).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
  expect(read.mock.calls[0]![0]).toBe(repository().manifest!.id);

  await act("Start Autopilot").click();
  await expect.element(strip()).toHaveTextContent("armada");
  expect(start).toHaveBeenCalledTimes(1);
  expect(start.mock.calls[0]![0]).toBe(repository().manifest!.id);
});

test("a Job sent leaves a strip, and Enter opens the Job the call named", async () => {
  const app = await cockpit();
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(strip()).toHaveTextContent(NAMING_BRIDGE);
  await expect.element(request()).toHaveValue("");

  // The Job's id is the one the call carries, and the Board's row for it is what the strip draws.
  const state = await app.api.state();
  const made = state.jobs.find((job) => job.title === NAMING_BRIDGE);
  expect(made).toBeDefined();
  expect(state.proposing?.job_id).toBe(made?.id);

  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("heading", { name: new RegExp(NAMING_BRIDGE.slice(0, 20)) })).toBeVisible();
});

test("Escape lets the strip go, and typing starts a new draft over it", async () => {
  await cockpit();
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(strip().query()).toBeNull();

  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  await userEvent.keyboard("x");
  expect(strip().query()).toBeNull();
  await expect.element(request()).toHaveValue("x");
});

test("the draft is kept when the panel is put back, and cleared by a send", async () => {
  await cockpit();
  await request().fill(NAMING_NOTHING);
  await expect.element(act("Dispatch")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(frame()).not.toHaveAttribute("data-open");
  await expect.element(request()).toHaveValue(NAMING_NOTHING);

  await request().click();
  await expect.element(frame()).toHaveAttribute("data-open");
  await expect.element(request()).toHaveValue(NAMING_NOTHING);

  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("Repository armada")).toBeVisible();
  await act("Dispatch").click();
  await expect.element(request()).toHaveValue("");
  await expect.element(frame()).not.toHaveAttribute("data-open");
});

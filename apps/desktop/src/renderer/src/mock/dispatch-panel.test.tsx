// The one Dispatch sheet through `App` on the mock Fleet's `quick-dispatch` scenario, four set-up repositories with
// armada busiest: `n` opens it directly, the Cockpit's tile and the Jobs page's New Job open the same one, and the
// repository is a row of chips with their keys, read from the words as Fleet reads them and given the keys when the
// act finds none chosen. Also the three kinds, the strip a send leaves, and the draft. The look is the walk's
// (`quick-dispatch-deck-button`); this holds what it decided.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const SCENARIO = "quick-dispatch";
const NAMING_BRIDGE = "Show the Drone's branch on each Bridge cockpit tile";
const NAMING_DOCS = "Add a docs-site page for the retry settings";
const NAMING_NOTHING = "Clean up the old retry logic";
const ARMADA = repository().root;
const STOREFRONT = "/Users/user/code/storefront";
const BILLING = "/Users/user/code/billing";

const frame = () => page.getByRole("region", { name: "Dispatch", exact: true });
const request = () => page.getByRole("textbox", { name: "Request" });
const row = () => frame().getByRole("radiogroup", { name: "Repository" });
const chip = (name: string) => frame().getByRole("radio", { name: `Repository ${name}` });
const reading = (name: string) => frame().getByRole("img", { name });
const act = (name: string) => frame().getByRole("button", { name, exact: true });
const strip = () => page.getByRole("group", { name: "Last dispatch" });
const palette = () => page.getByRole("dialog", { name: "Command palette" });
const tile = () => page.getByRole("region", { name: "Dispatch controls" }).getByRole("button", { name: "Dispatch" });

/** The chips' names, in the order the row draws them. */
const names = () => [...row().element().querySelectorAll("[role='radio'] .armada-dispatch-panel__name")].map((one) => one.textContent);

/** The Cockpit with the sheet open on an empty draft, from the key and nothing else. */
async function sheet() {
  const app = mount(SCENARIO);
  await onScreen();
  await userEvent.keyboard("n");
  await expect.element(request()).toBeVisible();
  return app;
}

test("N opens the sheet directly from the Cockpit, the caret in the request and no palette first", async () => {
  mount(SCENARIO);
  await onScreen();
  expect(request().query()).toBeNull();

  await userEvent.keyboard("n");
  await expect.element(request()).toBeVisible();
  await expect.element(request()).toHaveFocus();
  expect(palette().query()).toBeNull();
  // The kinds are on the band, and every set-up repository is a chip.
  await expect.element(frame().getByRole("tab", { name: "Job" })).toBeVisible();
  await expect.element(row()).toBeVisible();
});

test("the Cockpit's Dispatch tile opens the same sheet", async () => {
  mount(SCENARIO);
  await onScreen();
  await expect.element(tile()).toBeVisible();
  await tile().click();
  await expect.element(request()).toBeVisible();
  await expect.element(request()).toHaveFocus();
});

test("New Job on the Jobs page opens the sheet, on Job, and not the composer's page with a dropdown", async () => {
  mount(SCENARIO);
  await onScreen();
  await page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "Jobs" }).click();
  await page.getByRole("region", { name: "Jobs" }).getByRole("button", { name: "New Job" }).click();
  await expect.element(request()).toBeVisible();
  await expect.element(frame().getByRole("tab", { name: "Job" })).toHaveAttribute("aria-selected", "true");
  expect(page.getByRole("button", { name: "Choose a repository", exact: true }).query()).toBeNull();
});

test("a dispatch row chosen in the palette moves the words into the sheet in one press", async () => {
  mount(SCENARIO);
  await onScreen();
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await palette().getByRole("combobox").fill(NAMING_BRIDGE);
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(request()).toHaveValue(NAMING_BRIDGE);
  expect(palette().query()).toBeNull();
});

test("every set-up repository is a chip, ranked as Fleet ranks them, with nothing chosen until one is read or picked", async () => {
  await sheet();
  expect(names()).toEqual(["armada", "billing", "docs-site", "storefront"]);
  expect(row().element().querySelectorAll("[aria-checked='true']")).toHaveLength(0);
});

test("the words are read for their repository: it leads the row and is chosen, and a request naming none is left unresolved", async () => {
  await sheet();
  await request().fill(NAMING_DOCS);
  await expect.element(reading("Repository, finding")).toBeVisible();
  await expect.element(chip("docs-site")).toHaveAttribute("aria-checked", "true");
  // The one the words name comes first, then the rest as Fleet ranks them.
  expect(names()).toEqual(["docs-site", "armada", "billing", "storefront"]);

  await request().fill(NAMING_NOTHING);
  await expect.element(reading("Repository, not found")).toBeVisible();
  // Nothing asks until Dispatch is pressed.
  expect(frame().element().hasAttribute("data-asking")).toBe(false);
});

test("⌥2 picks the second chip, from inside the request", async () => {
  await sheet();
  await request().fill(NAMING_DOCS);
  await expect.element(chip("docs-site")).toHaveAttribute("aria-checked", "true");
  await userEvent.keyboard("{Alt>}2{/Alt}");
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await expect.element(chip("docs-site")).toHaveAttribute("aria-checked", "false");
  // The caret stays in the request, and the order did not move under the key.
  await expect.element(request()).toHaveFocus();
  expect(names()).toEqual(["docs-site", "armada", "billing", "storefront"]);
});

test("a repository read from the words is the one the Job is sent to", async () => {
  const app = await sheet();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  expect(propose).toHaveBeenCalledWith(NAMING_BRIDGE, [], ARMADA);
});

test("pressing the act with nothing chosen gives the row the keys, and a chip picked there sends", async () => {
  const app = await sheet();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_NOTHING);
  await expect.element(reading("Repository, not found")).toBeVisible();
  await act("Dispatch").click();
  // The frame takes the call's hue, the first chip has focus, and nothing was built.
  await expect.element(frame()).toHaveAttribute("data-asking");
  await expect.element(chip("armada")).toHaveFocus();
  expect(propose).not.toHaveBeenCalled();

  await userEvent.keyboard("{ArrowRight}");
  await expect.element(chip("billing")).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await expect.element(strip()).toBeVisible();
  expect(propose).toHaveBeenCalledWith(NAMING_NOTHING, [], BILLING);
});

test("the Job lands on the repository chosen, not the first the Board lists", async () => {
  const app = await sheet();
  await request().fill(NAMING_NOTHING);
  await act("Dispatch").click();
  await chip("storefront").click();
  await expect.element(strip()).toBeVisible();
  const state = await app.api.state();
  const made = state.jobs.find((job) => job.title === NAMING_NOTHING);
  expect(made?.owner_manifest_id).toBe("storefront");
});

test("an answer to words that have since changed is let go", async () => {
  const app = await sheet();
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
  await expect.element(reading("Repository, not found")).toBeVisible();
  answers[0]!({ manifest_id: repository().manifest!.id, likely: [repository().manifest!.id] });
  await new Promise((resume) => setTimeout(resume, 50));
  await expect.element(reading("Repository, not found")).toBeVisible();
  expect(chip("armada").element().getAttribute("aria-checked")).toBe("false");
});

test("a repository picked by hand beats the one read from the words", async () => {
  const app = await sheet();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await chip("storefront").click();
  await expect.element(chip("storefront")).toHaveAttribute("aria-checked", "true");
  // Words read again do not take the pick back.
  await request().fill(`${NAMING_BRIDGE} again`);
  await new Promise((resume) => setTimeout(resume, 800));
  await expect.element(chip("storefront")).toHaveAttribute("aria-checked", "true");
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  expect(propose).toHaveBeenCalledWith(`${NAMING_BRIDGE} again`, [], STOREFRONT);
});

// The owner, 13 Oct 2026: on All, a Session asks where it starts, as a Job does.
test("a Session on All that names no repository gives the row the keys, and the pick starts it there", async () => {
  const app = await sheet();
  const propose = vi.spyOn(app.api, "proposeFromRequest");
  await request().fill(NAMING_NOTHING);
  await userEvent.keyboard("{Tab}");
  await expect.element(frame().getByRole("tab", { name: "Session" })).toHaveAttribute("aria-selected", "true");
  await expect.element(reading("Repository, not found")).toBeVisible();
  await act("Start").click();
  await expect.element(frame()).toHaveAttribute("data-asking");
  await chip("storefront").click();
  await expect.element(page.getByRole("textbox", { name: "Message" })).toBeVisible();
  expect(propose).not.toHaveBeenCalled();
});

test("a Session whose words name a repository starts there without asking", async () => {
  await sheet();
  await request().fill(NAMING_BRIDGE);
  await userEvent.keyboard("{Tab}");
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Start").click();
  await expect.element(page.getByRole("textbox", { name: "Message" })).toBeVisible();
});

test("an Autopilot is read first, and started from what was read", async () => {
  const app = await sheet();
  const read = vi.spyOn(app.api, "proposeAutopilot");
  const start = vi.spyOn(app.api, "startAutopilot");
  await request().fill("Work through the open Bridge bugs until 7am");
  await frame().getByRole("tab", { name: "Autopilot" }).click();
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Read it").click();
  await expect.element(page.getByRole("textbox", { name: "Instruction" })).toBeVisible();
  expect(read).toHaveBeenCalledTimes(1);
  expect(read.mock.calls[0]![0]).toBe(repository().manifest!.id);

  await act("Start Autopilot").click();
  await expect.element(strip()).toHaveTextContent("armada");
  expect(start).toHaveBeenCalledTimes(1);
  expect(start.mock.calls[0]![0]).toBe(repository().manifest!.id);

  // Opening it from the strip puts the Cockpit's Autopilot bar up for that repository.
  expect(page.getByRole("region", { name: "Autopilot lane" }).query()).toBeNull();
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: "Autopilot lane" })).toBeVisible();
});

test("a Job sent leaves a strip, and Enter opens the Job the call named", async () => {
  const app = await sheet();
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Dispatch").click();
  await expect.element(strip()).toHaveTextContent(NAMING_BRIDGE);
  // The sheet has gone with the send, and the strip stands over the title bar's field.
  expect(frame().query()).toBeNull();

  // The Job's id is the one the call carries, and the Board's row for it is what the strip draws.
  const state = await app.api.state();
  const made = state.jobs.find((job) => job.title === NAMING_BRIDGE);
  expect(made).toBeDefined();
  expect(state.proposing?.job_id).toBe(made?.id);

  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("heading", { name: new RegExp(NAMING_BRIDGE.slice(0, 20)) })).toBeVisible();
});

test("Escape lets the strip go, and typing starts a new draft over it", async () => {
  await sheet();
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(strip().query()).toBeNull();

  await userEvent.keyboard("n");
  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  await userEvent.keyboard("x");
  expect(strip().query()).toBeNull();
  await expect.element(request()).toHaveValue("x");
});

test("Escape puts the sheet away and N brings the draft back, and a send clears it", async () => {
  await sheet();
  await request().fill(NAMING_NOTHING);
  await expect.element(act("Dispatch")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(frame().query()).toBeNull();
  expect(request().query()).toBeNull();

  // The words were kept: the key opens the sheet holding them, not an empty one.
  await userEvent.keyboard("n");
  await expect.element(request()).toHaveValue(NAMING_NOTHING);
  expect(palette().query()).toBeNull();
  await userEvent.keyboard("{Escape}");
  expect(frame().query()).toBeNull();

  // Typing over them and sending leaves nothing behind.
  await userEvent.keyboard("n");
  await expect.element(request()).toHaveValue(NAMING_NOTHING);

  await request().fill(NAMING_BRIDGE);
  await expect.element(chip("armada")).toHaveAttribute("aria-checked", "true");
  await act("Dispatch").click();
  await expect.element(strip()).toBeVisible();
  await userEvent.keyboard("{Escape}");
  expect(strip().query()).toBeNull();
  await userEvent.keyboard("n");
  await expect.element(request()).toHaveValue("");
});

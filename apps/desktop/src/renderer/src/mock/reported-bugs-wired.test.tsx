// Reported bugs on the mock Fleet, through `App`: the band under All and under armada and under no other repository, the ring
// going when a report is opened, Keep and Drop written through Fleet and heard back as `bug_reports.changed`, Start a Job
// opening the sheet in armada with the report as its brief, `K` and `D` acting only where a report holds focus, and the mark
// on the Cockpit row of a Job that reported one. The look is the walk's (`reported-bugs`); this holds what it decided.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { reportedBugs } from "./scenarios/reported-bugs";
import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const STOREFRONT = "/Users/user/code/storefront";
const ARMADA = "/Users/user/code/armada";

const issues = () => page.getByRole("region", { name: "Issues", exact: true });
const band = () => page.getByRole("listitem", { name: "Reported bugs", exact: true });
const reader = () => page.getByRole("region", { name: "Reported bug", exact: true });
const row = (title: RegExp) => issues().getByRole("button", { name: title });
const WATCH = /^A watch stayed live/;
const VERB = /^waiting_for refused/;
const SKETCH = /^show_sketch refused/;
const unread = () => document.querySelectorAll('[role="img"][aria-label="Unread"]').length;

const scenarioIn = (repository: string | null) => ({ ...reportedBugs, state: { ...reportedBugs.state, repository } });

async function open(repository: string | null = null) {
  const app = mount(scenarioIn(repository));
  await onScreen();
  await toPlace("Issues" as never);
  await expect.element(issues()).toBeVisible();
  return app;
}

test("the band is drawn under All", async () => {
  await open(null);
  await expect.element(band()).toBeVisible();
});

test("the band is drawn with armada picked", async () => {
  await open(ARMADA);
  await expect.element(band()).toBeVisible();
});

test("no band stands over another repository's issues", async () => {
  await open(STOREFRONT);
  await expect.element(issues().getByText("Coupon field accepts an expired code")).toBeVisible();
  expect(band().query()).toBeNull();
});

test("the reports are read from Fleet's list, and a report opened loses its ring", async () => {
  const app = mount(scenarioIn(null));
  const read = vi.spyOn(app.api, "listBugReports");
  await onScreen();
  await toPlace("Issues" as never);
  await expect.element(band()).toBeVisible();
  expect(read).toHaveBeenCalled();
  expect(unread()).toBe(2);

  await userEvent.click(row(WATCH));
  await expect.element(reader()).toBeVisible();
  expect(unread()).toBe(1);
});

test("Keep is written through Fleet, and Start a Job takes its place once Fleet answers", async () => {
  const app = await open();
  const keep = vi.spyOn(app.api, "keepBugReport");
  await userEvent.click(row(WATCH));
  await userEvent.click(reader().getByRole("button", { name: "Keep", exact: true }));

  await expect.poll(() => keep.mock.calls.length).toBe(1);
  expect(keep.mock.calls[0]![0]).toBe("report-watch-live");
  await expect.element(reader().getByRole("button", { name: "Start a Job", exact: true })).toBeVisible();
  expect(reader().getByRole("button", { name: "Keep", exact: true }).query()).toBeNull();
});

test("Drop is written through Fleet, and the report leaves the band", async () => {
  const app = await open();
  const drop = vi.spyOn(app.api, "dropBugReport");
  await userEvent.click(row(WATCH));
  await userEvent.click(reader().getByRole("button", { name: "Drop", exact: true }));

  await expect.poll(() => drop.mock.calls.length).toBe(1);
  expect(drop.mock.calls[0]![0]).toBe("report-watch-live");
  await expect.poll(() => row(WATCH).query()).toBeNull();
  expect(reader().query()).toBeNull();
});

test("a report dropped elsewhere leaves the band on bug_reports.changed, with no read of the list", async () => {
  const app = await open();
  await expect.element(row(SKETCH)).toBeVisible();
  const reread = vi.spyOn(app.api, "listBugReports");

  await app.api.dropBugReport("report-sketch-icon");

  await expect.poll(() => row(SKETCH).query()).toBeNull();
  expect(reread).not.toHaveBeenCalled();
});

test("Start a Job drafts the Job in armada and opens it holding the report as the brief, with no sheet", async () => {
  await open();
  await userEvent.click(row(WATCH));
  await userEvent.click(reader().getByRole("button", { name: "Keep", exact: true }));
  await userEvent.click(reader().getByRole("button", { name: "Start a Job", exact: true }));

  const job = page.getByRole("region", { name: /^Job: / });
  await expect.element(job).toBeVisible();
  await expect.element(job).toHaveTextContent("A watch stayed live after its pull request merged");
  await expect.element(job).toHaveTextContent("Steps");
  await expect.element(job).toHaveTextContent("Start a watch on a deploy with start_watch.");
  await expect.element(job).toHaveTextContent("fleet.watch_live: a live watch holds this Job open");
  expect(page.getByRole("region", { name: "Dispatch", exact: true }).query()).toBeNull();
  expect(page.getByRole("textbox", { name: "Request" }).query()).toBeNull();
});

test("K keeps the report in the reader, and does nothing on a report already kept", async () => {
  const app = await open();
  const keep = vi.spyOn(app.api, "keepBugReport");
  await userEvent.click(row(WATCH));
  reader().element().focus();
  await userEvent.keyboard("K");
  await expect.poll(() => keep.mock.calls.length).toBe(1);
  await expect.element(reader().getByRole("button", { name: "Start a Job", exact: true })).toBeVisible();
  await userEvent.keyboard("K");
  expect(keep).toHaveBeenCalledTimes(1);
});

test("D on a report row drops that report", async () => {
  const app = await open();
  const drop = vi.spyOn(app.api, "dropBugReport");
  row(VERB).element().focus();
  await userEvent.keyboard("D");

  await expect.poll(() => drop.mock.calls.length).toBe(1);
  expect(drop.mock.calls[0]![0]).toBe("report-waiting-verb");
  await expect.poll(() => row(VERB).query()).toBeNull();
});

test("D and K on an issue describe a query and keep nothing, since no report holds focus", async () => {
  const app = await open();
  const drop = vi.spyOn(app.api, "dropBugReport");
  const keep = vi.spyOn(app.api, "keepBugReport");
  await expect.element(issues().getByText("The Cockpit list drops a row while a Job is moving")).toBeVisible();
  issues().getByRole("button", { name: /The Cockpit list drops a row/ }).element().focus();
  await userEvent.keyboard("K");
  await userEvent.keyboard("D");

  await expect.element(page.getByRole("textbox", { name: "Describe a query" })).toBeVisible();
  expect(drop).not.toHaveBeenCalled();
  expect(keep).not.toHaveBeenCalled();
});

test("a Job that reported a bug wears the mark on its Cockpit row, which opens its report", async () => {
  mount(scenarioIn(null));
  await onScreen();
  await toPlace("Cockpit");
  const marks = page.getByRole("button", { name: "Reported a bug", exact: true });
  await expect.poll(() => marks.elements().length).toBe(2);

  await marks.first().hover();
  await expect.element(page.getByText("Reported a bug", { exact: true }).first()).toBeInTheDocument();
  await userEvent.click(marks.first());

  await expect.element(reader()).toBeVisible();
});

test("a Job with no report wears no mark", async () => {
  mount({ ...scenarioIn(null), draft: { ...reportedBugs.draft!, reportedBugs: [] } });
  await onScreen();
  await toPlace("Cockpit");
  await expect.element(page.getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
  expect(page.getByRole("button", { name: "Reported a bug", exact: true }).query()).toBeNull();
});

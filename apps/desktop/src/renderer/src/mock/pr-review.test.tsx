// The Reviews tab through `App`, on the `pr-review` scenario whose mock Fleet serves the shapes the real one does: the four
// lists read when the tab opens, a pull request's page with its threads, and a review posted as one call with its line
// comments placed by path, line and side. A refused review keeps what he wrote. Hand-run with the rest of `mock/`.

import type { WritePullRequest } from "@armada/protocol";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import type { MockForge } from "./review-forge";
import { s262PrReview } from "./scenarios/pr-review";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Pull requests" });
const reader = (number: number) => page.getByRole("region", { name: `Pull request #${number}` });
const yours = () => page.getByRole("region", { name: "Your review" });
const row = (number: number) => list().getByRole("button", { name: new RegExp(`^#${number} `) });

async function reviews() {
  const app = mount(s262PrReview);
  await onScreen();
  await page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "Reviews" }).click();
  await expect.element(list()).toBeVisible();
  return app;
}

/** A row of the list opens the pull request in the reader the Reviews page keeps, and no side panel opens over it. */
async function open(number: number) {
  await row(number).click();
  await expect.element(reader(number)).toBeVisible();
  await expect.element(page.getByRole("dialog", { name: `Pull request #${number}` })).not.toBeInTheDocument();
}

async function choose(name: string) {
  await list().getByRole("button", { name: "List" }).click();
  await page.getByRole("menuitem", { name }).click();
}

test("the tab lists the four lists as the forge answered them", async () => {
  await reviews();
  // Requested for Review is where it opens.
  await expect.element(row(1899)).toBeVisible();
  await expect.element(row(212)).toBeVisible();
  await expect.element(row(1903)).not.toBeInTheDocument();

  await choose("Team");
  await expect.element(row(1903)).toBeVisible();
  await expect.element(row(1907)).toBeVisible();
  // A pull request on two lists is on both, with the team that was asked named on the Team list's row.
  await expect.element(row(1899)).toBeVisible();
  await expect.element(list().getByRole("note", { name: "@Mele-Labs/bridge" })).toBeVisible();
  await expect.element(row(212)).not.toBeInTheDocument();

  await choose("Assigned");
  await expect.element(row(1908)).toBeVisible();
  await expect.element(row(1899)).toBeVisible();
  await expect.element(row(1903)).not.toBeInTheDocument();

  await choose("My PRs");
  await expect.element(row(1896)).toBeVisible();
  await expect.element(row(1892)).toBeVisible();
  await expect.element(row(1899)).not.toBeInTheDocument();
});

test("a review goes out as one call with its line comment placed, and then it is recorded", async () => {
  const app = await reviews();
  const wrote: WritePullRequest[] = [];
  const write = app.api.writeReview;
  app.api.writeReview = async (manifestId, one) => {
    wrote.push(one);
    return write(manifestId, one);
  };

  await open(1899);
  await expect.element(reader(1899)).toBeVisible();
  // The changes are in the explanation, which an agent reads once he has said it may run.
  await reader(1899).getByRole("region", { name: "Quick actions" }).getByRole("tab", { name: "How it works" }).click();
  await page.getByRole("dialog", { name: "How it works" }).getByRole("button", { name: "Run" }).click();
  await expect.element(reader(1899).getByRole("region", { name: "Theme: Fleet keeps the last read" })).toBeVisible();
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Comments" }).click();
  const serve = reader(1899).getByRole("article", { name: "Change: Serve it while it is fresh" });
  const form = reader(1899).getByRole("form", { name: "Comment" });
  const addComment = async (words: string) => {
    await reader(1899).getByRole("textbox", { name: "Comment" }).fill(words);
    await form.getByRole("button", { name: "Add to review" }).click();
  };

  // A line of the hunk, and then the change itself.
  await serve.getByRole("button", { name: "Show change" }).click();
  await serve.getByRole("button", { name: /Comment on \+ let mut held = self\.held\.lock/ }).click();
  await addComment("Take the lock after the call.");
  await serve.getByRole("button", { name: "Comment on Serve it while it is fresh" }).click();
  await addComment("This whole method wants a test.");
  await expect.element(yours().getByRole("list", { name: "Pending comments" })).toBeVisible();

  await yours().getByRole("textbox", { name: "Summary" }).fill("The lock goes after the call.");
  await yours().getByRole("tab", { name: "Request changes" }).click();
  await yours().getByRole("button", { name: "Submit review" }).click();

  await expect.element(yours().getByRole("list", { name: "Submitted reviews" })).toBeVisible();
  await expect.element(yours().getByRole("list", { name: "Pending comments" })).not.toBeInTheDocument();
  await expect.element(list().getByRole("img", { name: "Changes requested" })).toBeVisible();

  expect(wrote).toHaveLength(1);
  const sent = wrote[0]!;
  expect(sent).toMatchObject({ pull_request: 1899, kind: "request_changes", body: "The lock goes after the call." });
  expect(sent.commit).toBeTruthy();
  // A line is a place in a file, not an index into the drawn lines; the change's own comment is on its file.
  expect(sent.comments).toEqual([
    { kind: "line", path: expect.stringContaining("/"), line: 31, side: "right", body: "Take the lock after the call." },
    { kind: "file", path: expect.stringContaining("/"), body: "This whole method wants a test." },
  ]);
});

test("a review the forge refuses keeps the comment and the summary, and says why", async () => {
  const app = await reviews();
  (app.api as typeof app.api & Pick<MockForge, "refuseNextWrite">).refuseNextWrite("Can not approve your own pull request");

  await open(1899);
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Comments" }).click();
  await yours().getByRole("textbox", { name: "Summary" }).fill("Looks right to me.");
  await yours().getByRole("tab", { name: "Approve" }).click();
  await yours().getByRole("button", { name: "Submit review" }).click();

  await expect.element(yours().getByRole("alert")).toHaveTextContent("Can not approve your own pull request");
  await expect.element(yours().getByRole("textbox", { name: "Summary" })).toHaveValue("Looks right to me.");
  await expect.element(yours().getByRole("list", { name: "Submitted reviews" })).not.toBeInTheDocument();

  // Pressed again, the forge takes it.
  await yours().getByRole("button", { name: "Submit review" }).click();
  await expect.element(yours().getByRole("list", { name: "Submitted reviews" })).toBeVisible();
  await expect.element(yours().getByRole("alert")).not.toBeInTheDocument();
});

test("a tool that found nothing draws its picture and says what it checked", async () => {
  await reviews();
  await open(212);
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Risks" }).click();
  await page.getByRole("dialog", { name: "Risks" }).getByRole("button", { name: "Run" }).click();
  const results = page.getByRole("region", { name: "Risks results" });
  await expect.element(results.getByRole("img", { name: "Nothing found" })).toBeVisible();
  await expect.element(results.getByText(/^No risks in \d+ changes?$/)).toBeVisible();
});

test("a pull request one of his Sessions holds opens with what was kept for it, and no run is asked for", async () => {
  await reviews();
  await choose("My PRs");
  await open(1896);
  await expect.element(reader(1896).getByRole("button", { name: "Job: Fix the reconnect wait" })).toBeVisible();
  const how = reader(1896).getByRole("region", { name: "How it works" });
  await expect.element(how.getByRole("img", { name: /^Kept/ })).toBeVisible();
  await reader(1896).getByRole("region", { name: "Quick actions" }).getByRole("tab", { name: "Why" }).click();
  await expect.element(reader(1896).getByRole("region", { name: "Why" }).getByRole("img", { name: /^Kept/ })).toBeVisible();
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("a run that fails shows Fleet's reason where the result would be, and Run again starts another", async () => {
  const app = await reviews();
  (app.api as typeof app.api & Pick<MockForge, "failNextRun">).failNextRun("The answer was not valid JSON");
  await open(212);
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Risks" }).click();
  await page.getByRole("dialog", { name: "Risks" }).getByRole("button", { name: "Run" }).click();
  const results = page.getByRole("region", { name: "Risks results" });
  await expect.element(results.getByRole("alert")).toHaveTextContent("The answer was not valid JSON");
  await expect.element(results.getByRole("img", { name: "Nothing found" })).not.toBeInTheDocument();

  await results.getByRole("button", { name: "Run again" }).click();
  await page.getByRole("dialog", { name: "Risks" }).getByRole("button", { name: "Run" }).click();
  await expect.element(results.getByRole("img", { name: "Nothing found" })).toBeVisible();
  await expect.element(results.getByRole("alert")).not.toBeInTheDocument();
});

test("a tool whose name is taken is refused in Fleet's words, beside Save", async () => {
  await reviews();
  await open(1899);
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Add a tool" }).click();
  const sheet = page.getByRole("dialog", { name: "Add a tool" });
  await sheet.getByRole("textbox", { name: "Name" }).fill("Test gaps");
  await sheet.getByRole("textbox", { name: "What it checks" }).fill("What no test would notice.");
  await sheet.getByRole("button", { name: "Save" }).click();
  await expect.element(sheet.getByRole("alert")).toHaveTextContent("test-gaps.md is already in the Kit");
  await expect.element(sheet.getByRole("list", { name: "Left out" })).toBeVisible();
});

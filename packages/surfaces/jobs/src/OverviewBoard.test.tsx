// The Overview's fallback cards, mounted — the ones a Job draws while its
// canvas has nothing to draw (a read that failed or has not answered).
//
// **A block comment written as JSX text is drawn.** One sat unbraced inside the
// fragment and showed as prose above the Brief on the owner's own screen.

import { Button } from "@armada/components";
import type { CommanderPanelProps, NowSlotAsk } from "@armada/components";
import { Eye, Stamp } from "lucide-react";
import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { OverviewBoard } from "./OverviewBoard";
import { mount } from "@armada/screens/src/mounted";

test("the fallback cards draw no raw comment text", async () => {
  mount(
    <OverviewBoard
      briefAbsent="Fleet did not answer"
      workflowAbsent="Fleet did not answer"
      planAbsent="Fleet did not answer"
      pulse={[]}
      settings=""
      onOpenTab={() => undefined}
    />
  );
  await expect.element(page.getByText("Fleet did not answer").first()).toBeVisible();
  const text = document.querySelector(".armada-overview-board")?.textContent ?? "";
  expect(text).not.toContain("/*");
  expect(text).not.toContain("*/");
});

// **The lead is gone from Overview, and the Commander is in its place** (owner, 11 Oct 2026,
// decision record "the Commander takes the lead's place"). What the lead held to answer
// is a card in the Now panel, in the Cockpit's own call-card frame.

const COMMANDER: CommanderPanelProps = { jobId: "j1", state: "idle", rows: [], onSend: () => undefined };

const APPROVAL: NowSlotAsk = {
  key: "lead",
  kind: "slot",
  band: "Approve dispatch",
  icon: Stamp,
  title: "Waiting for your approval",
  children: <Button variant="primary">Approve dispatch</Button>,
};

const REVIEW: NowSlotAsk = {
  key: "lead",
  kind: "slot",
  band: "Review",
  icon: Eye,
  title: "Ready for review",
  children: <p>All 3 Checks passed</p>,
};

const FILLED = { briefAbsent: "Fleet did not answer", workflowAbsent: "Fleet did not answer", planAbsent: "Fleet did not answer", pulse: [], settings: "", onOpenTab: () => undefined };

const box = (selector: string): DOMRect => {
  const found = document.querySelector(selector);
  if (found === null) throw new Error(`nothing matched ${selector}`);
  return found.getBoundingClientRect();
};

test("the Commander is the first thing on the board, across its width, over the canvas and Now", async () => {
  mount(<OverviewBoard {...FILLED} commander={COMMANDER} now={{ running: [{ key: "r", of: "drone", name: "Implement Drone", state: "running", onOpen: () => undefined }] }} />);
  await expect.element(page.getByRole("region", { name: "Commander" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
  const board = document.querySelector(".armada-overview-board");
  expect(board?.firstElementChild).toBe(page.getByRole("region", { name: "Commander" }).element());
  // The lead's place, and no lead.
  expect(document.querySelector(".armada-lead")).toBeNull();
  const commander = box(".armada-commander");
  expect(commander.width).toBeGreaterThanOrEqual(box(".armada-overview-board").width - 2);
  expect(commander.bottom).toBeLessThanOrEqual(box(".armada-overview-board__with-now").top + 1);
});

test("a Job with no Commander draws none, and nothing in its place", async () => {
  mount(<OverviewBoard {...FILLED} now={{ running: [{ key: "r", of: "drone", name: "Implement Drone", state: "running", onOpen: () => undefined }] }} />);
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
  expect(page.getByRole("region", { name: "Commander" }).query()).toBeNull();
  expect(document.querySelector(".armada-overview-board")?.firstElementChild).toBe(document.querySelector(".armada-overview-board__with-now"));
});

test("an approval is an ask in the Now panel, in the call card's frame, and its act is there", async () => {
  mount(<OverviewBoard {...FILLED} commander={COMMANDER} now={{ asks: [APPROVAL] }} />);
  const now = page.getByRole("region", { name: "Now" });
  const card = now.getByRole("region", { name: "Approve dispatch: Waiting for your approval" });
  await expect.element(card).toBeVisible();
  await expect.element(card.getByRole("heading", { name: "Waiting for your approval" })).toBeVisible();
  await expect.element(card.getByRole("button", { name: "Approve dispatch" })).toBeVisible();
  expect(now.element().contains(card.element())).toBe(true);
  expect(card.element().classList.contains("armada-callcard")).toBe(true);
  // Beside the canvas, under the Commander: not across the board.
  expect(box(".armada-now .armada-callcard").width).toBeLessThan(box(".armada-overview-board").width / 2);
});

test("the Now panel cannot be put away while it holds an ask, and can be when it holds none", async () => {
  mount(<OverviewBoard {...FILLED} now={{ asks: [APPROVAL] }} />);
  await expect.element(page.getByRole("region", { name: "Approve dispatch: Waiting for your approval" })).toBeVisible();
  expect(page.getByRole("button", { name: "Hide now" }).query()).toBeNull();

  mount(<OverviewBoard {...FILLED} now={{ running: [{ key: "r", of: "drone", name: "Implement Drone", state: "running", onOpen: () => undefined }] }} />);
  await expect.element(page.getByRole("button", { name: "Hide now" })).toBeVisible();
  await page.getByRole("button", { name: "Hide now" }).click();
  await expect.element(page.getByRole("button", { name: "Show now" })).toBeVisible();
  // Hidden is the window's to remember, so leave it as found.
  await page.getByRole("button", { name: "Show now" }).click();
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
});

test("a hidden panel comes back when an ask arrives in it", async () => {
  mount(<OverviewBoard {...FILLED} now={{ running: [{ key: "r", of: "drone", name: "Implement Drone", state: "running", onOpen: () => undefined }] }} />);
  await page.getByRole("button", { name: "Hide now" }).click();
  await expect.element(page.getByRole("button", { name: "Show now" })).toBeVisible();
  mount(<OverviewBoard {...FILLED} now={{ asks: [APPROVAL] }} />);
  await expect.element(page.getByRole("region", { name: "Now" })).toBeVisible();
  expect(page.getByRole("button", { name: "Show now" }).query()).toBeNull();
  // Leave the window's memory as found.
  mount(<OverviewBoard {...FILLED} now={{ running: [{ key: "r", of: "drone", name: "Implement Drone", state: "running", onOpen: () => undefined }] }} />);
  await page.getByRole("button", { name: "Show now" }).click();
});

test("a review at awaiting_review takes the board's width, under the Commander and over the canvas", async () => {
  mount(<OverviewBoard {...FILLED} commander={COMMANDER} wide={REVIEW} now={{ running: [{ key: "r", of: "drone", name: "Implement Drone", state: "running", onOpen: () => undefined }] }} />);
  const wide = page.getByRole("group", { name: "Asks you" });
  const card = wide.getByRole("region", { name: "Review: Ready for review" });
  await expect.element(card).toBeVisible();
  const board = box(".armada-overview-board");
  const card_ = box(".armada-overview-board__wide .armada-callcard");
  expect(card_.width).toBeGreaterThanOrEqual(board.width - 2);
  expect(card_.top).toBeGreaterThanOrEqual(box(".armada-commander").bottom - 1);
  expect(card_.bottom).toBeLessThanOrEqual(box(".armada-overview-board__with-now").top + 1);
  // Not also in Now's column.
  expect(page.getByRole("region", { name: "Now" }).getByRole("region", { name: "Review: Ready for review" }).query()).toBeNull();
});

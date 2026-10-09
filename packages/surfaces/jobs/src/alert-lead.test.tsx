// Alerts in the lead of a Job's Overview: one row for each Job `list_alerts` names, with the Trigger
// its row's bell is about, a mark and a tooltip, and a press that opens the Job where the Trigger fired.

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { AlertList, JobAlert, JobSummary } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";

import { alertRowsOf } from "./alert-rows";
import { JobLead } from "./JobLead";

afterEach(() => unmount());

const ALERT: JobAlert = { kind: "held", trigger: "deploy_qa", when: "pr_opened", step: "handoff" };
const job = (id: string, over: Partial<JobSummary> = {}) => ({ id, status: "awaiting_review", ...over }) as JobSummary;
const listed = (id: string, handle: string) => ({ job_id: id, handle, status: "awaiting_review" });

test("a listed Job whose row carries a Trigger's alert is a row, blocked first, and one with none is not", () => {
  const list: AlertList = { blocked: [listed("b", "2-b")], waiting: [listed("a", "1-a"), listed("gate", "3-gate")] };
  const rows = alertRowsOf(list, [job("a", { alert: ALERT }), job("b", { alert: { ...ALERT, kind: "failed" } }), job("gate")]);
  expect(rows.map((one) => [one.job, one.name, one.alert.kind])).toEqual([
    ["b", "2-b", "failed"],
    ["a", "1-a", "held"],
  ]);
});

test("the lead names the Job and the Trigger with the bell's mark, and a press opens it", async () => {
  const onOpen = vi.fn();
  mount(<JobLead said="Nothing needs you" because="" alerts={[{ job: "a", name: "1-a", alert: ALERT, onOpen }]} />);
  await expect.element(page.getByRole("list", { name: "Alerts" })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Held, deploy_qa, PR opened, handoff" })).toBeVisible();
  await page.getByRole("button", { name: "1-a, deploy_qa" }).click();
  expect(onOpen).toHaveBeenCalledTimes(1);
});

test("with no alert the lead draws no list, and no row draws a count", async () => {
  mount(<JobLead said="Nothing needs you" because="" alerts={[]} />);
  expect(page.getByRole("list", { name: "Alerts" }).elements()).toHaveLength(0);
  unmount();
  mount(<JobLead said="Nothing needs you" because="" alerts={[{ job: "a", name: "one", alert: ALERT }]} />);
  await expect.element(page.getByRole("list", { name: "Alerts" })).toBeVisible();
  expect(page.getByRole("list", { name: "Alerts" }).element().textContent).not.toMatch(/\d/);
});

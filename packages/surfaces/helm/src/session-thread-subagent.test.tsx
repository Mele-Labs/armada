// A subagent the agent's turn waits on is a row of its own in the thread, and a message sent
// meanwhile says it is queued, where it used to look ignored (armada-d3h.1).

import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { SessionThread } from "@armada/components";
import type { SessionThreadRow } from "@armada/components";
import type { ToolSubagent } from "@armada/protocol";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const TASK = "Make the Job browser thread resizable";

const subagent = (change: Partial<ToolSubagent> = {}): ToolSubagent => ({
  id: "agent:a1",
  agent_type: "implementer",
  task: TASK,
  latest: "Edit JobBrowserPane.tsx",
  state: "running",
  since: new Date(Date.now() - 65_000).toISOString(),
  ...change,
});

const tool = (one: ToolSubagent): SessionThreadRow => ({ id: "t1", at: "10:00:00", kind: "tool", text: `Agent ${TASK}`, subagent: one });

const thread = (rows: SessionThreadRow[], onOpenSubagent?: (id: string) => void) => (
  <SessionThread sessionId="a" rows={rows} onAnswer={() => {}} onOpenSession={() => {}} {...(onOpenSubagent === undefined ? {} : { onOpenSubagent })} />
);

const ran = () => document.querySelector(".armada-session-subagent__ran")?.textContent;

test("a running subagent is a button that names its task, shows its type, its latest call and how long it has run, and opens its thread", async () => {
  const open = vi.fn();
  mount(thread([tool(subagent())], open));

  const row = page.getByRole("button", { name: `Subagent ${TASK}, running` });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByText("implementer")).toBeVisible();
  await expect.element(row.getByText("Edit JobBrowserPane.tsx")).toBeVisible();
  expect(ran()).toMatch(/^1m \d\ds$/);

  await row.click();
  expect(open).toHaveBeenCalledOnce();
  expect(open).toHaveBeenCalledWith("agent:a1");
});

test("a running subagent's elapsed time moves on its own", async () => {
  mount(thread([tool(subagent())], () => {}));

  await expect.element(page.getByRole("button", { name: `Subagent ${TASK}, running` })).toBeVisible();
  const first = ran();
  expect(first).toMatch(/^1m \d\ds$/);
  await vi.waitFor(() => expect(ran()).not.toBe(first), { timeout: 3000 });
});

test.each([
  ["cut_off", "cut off"],
  ["finished", "finished"],
] as const)("a %s subagent is named %s, shows how long it ran and does not tick", async (state, said) => {
  mount(thread([tool(subagent({ state, since: "2026-10-29T10:00:00Z", ended: "2026-10-29T10:12:30Z" }))], () => {}));

  await expect.element(page.getByRole("button", { name: `Subagent ${TASK}, ${said}` })).toBeVisible();
  expect(page.getByRole("button", { name: `Subagent ${TASK}, running` }).query()).toBeNull();
  expect(ran()).toBe("12m 30s");

  // Longer than the clock's tick: a running row would have moved on, and would read hours from `since`.
  await new Promise((done) => setTimeout(done, 1500));
  expect(ran()).toBe("12m 30s");
});

test("a subagent row is no press where nothing opens it", async () => {
  mount(thread([tool(subagent())]));

  await expect.element(page.getByText(TASK)).toBeVisible();
  expect(page.getByRole("button", { name: `Subagent ${TASK}, running` }).query()).toBeNull();
});

test("a tool row without a subagent is not drawn as one", async () => {
  mount(thread([{ id: "t1", at: "10:00:00", kind: "tool", text: "Read src/lib.rs" }], () => {}));

  await expect.element(page.getByText("Read lib.rs")).toBeVisible();
  expect(document.querySelector(".armada-session-subagent")).toBeNull();
});

const said = (queued: boolean): SessionThreadRow => ({ id: "m1", at: "10:00:00", kind: "message", from: "you", text: "also check the tests", ...(queued ? { queued: true as const } : {}) });

test("a message held until the subagent returns says Queued, with what that means", async () => {
  mount(thread([said(true)]));

  await expect.element(page.getByText("also check the tests")).toBeVisible();
  await expect.element(page.getByText("Queued", { exact: true })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Queued until the subagent returns" })).toBeVisible();
});

test("a message that is not held draws no Queued mark", async () => {
  mount(thread([said(false)]));

  await expect.element(page.getByText("also check the tests")).toBeVisible();
  expect(page.getByText("Queued", { exact: true }).query()).toBeNull();
  expect(page.getByRole("img", { name: "Queued until the subagent returns" }).query()).toBeNull();
});

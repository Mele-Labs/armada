// The Usage page reads what the tokens were spent on through `window.armada`: the Spent on view appears when Fleet answers,
// its Jobs under the titles Bridge holds, and a Fleet that does not answer offers no such view.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";

import { usageSummaryNow } from "./mock/usage-summary";
import { wireSpentNow } from "./mock/usage-spent";
import { spentOf } from "./usage-spent-source";
import { UsageSurface } from "./usage-surface";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const unanswered = {
  ok: false,
  outcome: {
    ok: false,
    why: "not_connected",
  },
} as const;

async function mount(spent: unknown) {
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = {
    getUsageSummary: vi.fn(async (tz: number) => ({
      ok: true,
      value: usageSummaryNow(tz),
    })),
    getUsageRules: vi.fn(async () => unanswered),
    getUsageTime: vi.fn(async () => unanswered),
    getUsageHours: vi.fn(async () => unanswered),
    getUsageSpent: vi.fn(async () => spent),
  };
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () =>
    root.render(
      <UsageSurface
        state={{
          connection: {
            state: "connected",
          } as never,
          sessions: {
            state: "read",
            sessions: [],
          },
        }}
      />,
    ),
  );
  roots.push({
    root,
    host,
  });
  return host;
}

const tabs = (host: HTMLElement) =>
  [
    ...host.querySelectorAll('[role="tab"]'),
  ].map((one) => one.textContent);

test("Fleet's answer offers the Spent on view, and no answer offers none", async () => {
  expect(
    tabs(
      await mount({
        ok: true,
        value: wireSpentNow(),
      }),
    ),
  ).toContain("Spent on");
  expect(tabs(await mount(unanswered))).not.toContain("Spent on");
});

test("each window is labelled and each Job is under its title, else the end of its id", () => {
  const wire = wireSpentNow();
  const [first] = wire.windows[0]!.jobs;
  const titled = new Map([
    [
      first!.session_id,
      "Debug Job 14",
    ],
  ]);
  const drawn = spentOf(wire, titled);
  expect(drawn.windows.map((one) => one.label)).toEqual([
    "24 hours",
    "7 days",
    "14 days",
  ]);
  expect(drawn.windows[0]!.jobs[0]!.title).toBe("Debug Job 14");
  const other = drawn.windows[0]!.jobs[1]!;
  expect(other.title).toBe(other.id.slice(-6));
  expect(drawn.windows[1]!.total).toEqual(wire.windows[1]!.total);
});

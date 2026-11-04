// The Usage page reads where the time went through `window.armada`: the Time view and the Builds panel appear when Fleet
// answers, a build's tokens are asked for its own run, and a window Fleet does not answer for draws neither.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { usageTime as scenario } from "./mock/scenarios/usage-time";
import { jobTimeNow } from "./mock/job-time";
import { usageBuildsNow } from "./mock/usage-builds";
import { usageHoursNow } from "./mock/usage-hours";
import { usageSummaryNow } from "./mock/usage-summary";
import { wireHours, wireTime } from "./mock/usage-time-wire";
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

const now = Date.now();
const builds = usageBuildsNow(now);
const unanswered = {
  ok: false,
  outcome: {
    ok: false,
    why: "not_connected",
  },
} as const;

/** What Fleet answers: the mock's figures as they go on the wire, and the summary for a build's run as it has it. */
const fleet = () => ({
  getUsageSummary: vi.fn(async (tz: number, window?: { since: string; until: string }) => ({
    ok: true,
    value: {
      ...usageSummaryNow(tz, now),
      ...(window === undefined
        ? {}
        : {
            total: builds.find((one) => one.started === window.since)!.usage!,
          }),
    },
  })),
  getUsageRules: vi.fn(async () => unanswered),
  getUsageTime: vi.fn(async () => ({
    ok: true,
    value: wireTime(jobTimeNow(), now),
  })),
  getUsageHours: vi.fn(async () => ({
    ok: true,
    value: wireHours(usageHoursNow(now), builds),
  })),
});

let armada: ReturnType<typeof fleet>;
beforeEach(() => {
  armada = fleet();
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = armada;
});

async function mount() {
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

test("the Time view and the Builds panel are drawn from what Fleet answers", async () => {
  const host = await mount();

  expect(armada.getUsageTime).toHaveBeenCalledTimes(1);
  expect(armada.getUsageHours).toHaveBeenCalledTimes(1);
  expect(host.querySelector('[role="tab"][aria-selected]')).not.toBeNull();
  expect([
    ...host.querySelectorAll('[role="tab"]'),
  ].map((one) => one.textContent)).toContain("Time");
  expect(host.querySelectorAll('[data-builds] [role="option"]')).toHaveLength(builds.length);
});

test("each build's tokens are asked for its own run, newest first", async () => {
  await mount();

  const asked = armada.getUsageSummary.mock.calls.filter(([, window]) => window !== undefined);
  expect(asked).toHaveLength(builds.length);
  const newest = builds[0]!;
  expect(asked[0]![1]).toEqual({
    since: newest.started,
    until: wireHours(usageHoursNow(now), builds).until,
  });
  const older = builds[1]!;
  expect(asked[1]![1]).toEqual({
    since: older.started,
    until: newest.started,
  });
});

test("a window Fleet does not answer for draws no Time view and no Builds panel, and the Tokens page is as it was", async () => {
  armada.getUsageTime.mockResolvedValue(unanswered as never);
  armada.getUsageHours.mockResolvedValue(unanswered as never);
  const host = await mount();

  expect([
    ...host.querySelectorAll('[role="tab"]'),
  ].map((one) => one.textContent)).not.toContain("Time");
  expect(host.querySelector("[data-builds]")).toBeNull();
  expect(host.querySelector(".armada-usage__band")).not.toBeNull();
});

test("the mock scenario answers through the same path, and its builds each get their own tokens", async () => {
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = scenario.behaves!({} as never);
  const host = await mount();

  expect([
    ...host.querySelectorAll('[role="tab"]'),
  ].map((one) => one.textContent)).toContain("Time");
  expect(host.querySelectorAll('[data-builds] [role="option"]')).toHaveLength(builds.length);
});

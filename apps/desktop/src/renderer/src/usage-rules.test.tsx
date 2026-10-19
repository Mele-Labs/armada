// The Usage page's rules panel over Fleet's answer: how a rule reads, the slots left empty, what 1 to 4 and Esc do, and
// the line a picked rule shows. It draws what it is handed, so the page is mounted bare, with the mock's answer.

import { UsagePage } from "@armada/components";
import type { UsageRules } from "@armada/protocol";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";

import { usageRulesNow } from "./mock/usage-rules";
import { JOB_TITLES, usageSummaryNow } from "./mock/usage-summary";

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function mount(rules: UsageRules | undefined, titles: ReadonlyMap<string, string> = JOB_TITLES): HTMLElement {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  flushSync(() => root.render(<UsagePage summary={usageSummaryNow(0)} titles={titles} rules={rules} />));
  return host;
}

const press = async (key: string) => {
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
  await new Promise((done) => setTimeout(done, 30));
};
const panel = (host: HTMLElement) => host.querySelector('[aria-label="Usage rules"]');
const rules = (host: HTMLElement) => [...(panel(host)?.querySelectorAll('[aria-label="Rules"] > li') ?? [])];
const trips = (host: HTMLElement) => [...(panel(host)?.querySelectorAll(".armada-usage__trip") ?? [])];

test("no panel is drawn until Fleet has answered", () => {
  expect(panel(mount(undefined))).toBeNull();
});

test("each rule reads with its limit from the answer, and an off rule is marked off", () => {
  const host = mount(usageRulesNow());
  expect(rules(host).map((one) => one.querySelector(".armada-usage__limit")?.textContent)).toEqual([
    "300k, or 150k at 20 turns/h",
    "50% of the last hour",
    "under 70% over 10 turns",
    "20 turns only reading",
  ]);
  expect(rules(host).map((one) => one.getAttribute("aria-label")?.split(", ")[1])).toEqual(["on", "on", "off", "on"]);
  expect(trips(host)).toHaveLength(0);
});

test("a limit that moved is the one drawn", () => {
  const moved = usageRulesNow();
  moved.rules[0]!.limit = { value: 200, unit: "thousand tokens" };
  expect(rules(mount(moved))[0]!.querySelector(".armada-usage__limit")?.textContent).toBe("200k, or 100k at 20 turns/h");
});

test("1 to 4 pick a rule and list its trips, newest first, and Esc clears the pick", async () => {
  const host = mount(usageRulesNow());
  await press("4");
  expect(rules(host)[3]!.getAttribute("aria-selected")).toBe("true");
  expect(trips(host).map((one) => one.querySelector(".armada-usage__title")?.textContent)).toEqual(["Debug Job 14", "Cockpit Updates", "Jobs -> Session Merger"]);
  await press("1");
  expect(trips(host)).toHaveLength(5);
  await press("Escape");
  expect(rules(host).every((one) => one.getAttribute("aria-selected") === "false")).toBe(true);
  expect(trips(host)).toHaveLength(0);
});

test("a rule with no trips lists nothing when picked, and a key past the last rule does nothing", async () => {
  const host = mount(usageRulesNow());
  await press("3");
  expect(rules(host)[2]!.getAttribute("aria-selected")).toBe("true");
  expect(panel(host)!.querySelector(".armada-usage__trips")).toBeNull();
  expect(panel(host)!.querySelector(".armada-usage__tripheads")).toBeNull();
  await press("5");
  expect(rules(host)[2]!.getAttribute("aria-selected")).toBe("true");
});

test("a trip not yet measured leaves its after and its change empty", async () => {
  const host = mount(usageRulesNow());
  await press("1");
  const first = trips(host)[0]!;
  const figs = [...first.querySelectorAll(".armada-usage__fig")].map((one) => one.textContent);
  expect(figs).toEqual(["2h", "351k", "11.8", ""]);
  expect(first.querySelector(".armada-usage__delta")?.textContent).toBe("");
});

test("a measured trip draws its change with a name, and a small move reads as none", async () => {
  const host = mount(usageRulesNow());
  await press("4");
  const labels = trips(host).map((one) => one.querySelector(".armada-usage__delta")?.getAttribute("aria-label"));
  expect(labels).toEqual(["Down 60% in the burn per hour", "No change in the burn per hour", "Down 59% in the burn per hour"]);
});

test("a Job with no title shows its short id", async () => {
  const host = mount(usageRulesNow(), new Map());
  await press("2");
  const title = trips(host)[0]!.querySelector(".armada-usage__title");
  expect(title?.textContent).toBe(usageRulesNow().rules[1]!.trips[0]!.session_id.slice(-6));
  expect(title?.hasAttribute("data-id")).toBe(true);
});

test("the picked rule shows what the Job is told, with the newest trip's figure", async () => {
  const host = mount(usageRulesNow());
  expect(host.querySelector(".armada-usage__tell")).toBeNull();
  await press("2");
  expect(host.querySelector(".armada-usage__tell")?.textContent).toBe("Subagents burned 87% of your last hour, on opus. Pass model sonnet on reading and searching agents.");
  await press("3");
  expect(host.querySelector(".armada-usage__tell")?.textContent).toContain("Cache hit rate is 54% over your last 10 turns.");
});

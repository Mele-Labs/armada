import { expect, test } from "vitest";

import { DASHBOARD_TABS, dashboardTabNamed, dashboardTabOf, sectionsOfTab } from "./dashboard";

test("a notification's section lands on the tab that holds it", () => {
  expect(dashboardTabOf("needs-you")).toBe("running");
  expect(dashboardTabOf("running")).toBe("running");
  expect(dashboardTabOf("queued")).toBe("running");
  expect(dashboardTabOf("recently-ended")).toBe("done");
  expect(dashboardTabOf("done")).toBe("done");
});

test("every section is on exactly one tab", () => {
  const all = [...sectionsOfTab("running"), ...sectionsOfTab("done")];
  expect([...all].sort()).toEqual(["done", "needs-you", "other", "queued", "recently-ended", "running"]);
});

test("the filters are Active, Done and Bays, and none is Your move", () => {
  expect(DASHBOARD_TABS.map((tab) => tab.label)).toEqual(["Active", "Done", "Bays"]);
});

test("a stored tab nobody recognises opens Active, and Your move's old id is Active", () => {
  expect(dashboardTabNamed(null)).toBe("running");
  expect(dashboardTabNamed("tiles")).toBe("running");
  expect(dashboardTabNamed("command-central")).toBe("running");
  expect(dashboardTabNamed("done")).toBe("done");
  expect(dashboardTabNamed("bays")).toBe("bays");
});

import { expect, test } from "vitest";

import { dashboardTabNamed, dashboardTabOf, sectionsOfTab } from "./dashboard";

test("a notification's section lands on the tab that holds it", () => {
  expect(dashboardTabOf("needs-you")).toBe("command-central");
  expect(dashboardTabOf("running")).toBe("running");
  expect(dashboardTabOf("queued")).toBe("running");
  expect(dashboardTabOf("recently-ended")).toBe("done");
  expect(dashboardTabOf("done")).toBe("done");
});

test("every section is on exactly one tab", () => {
  const all = [...sectionsOfTab("command-central"), ...sectionsOfTab("running"), ...sectionsOfTab("done")];
  expect([...all].sort()).toEqual(["done", "needs-you", "other", "queued", "recently-ended", "running"]);
});

test("a stored tab nobody recognises opens Command Central", () => {
  expect(dashboardTabNamed(null)).toBe("command-central");
  expect(dashboardTabNamed("tiles")).toBe("command-central");
  expect(dashboardTabNamed("done")).toBe("done");
});

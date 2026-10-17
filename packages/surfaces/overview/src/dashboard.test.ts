import { expect, test } from "vitest";

import { dashboardTabNamed } from "./dashboard";

test("a stored tab nobody recognises opens Command Central", () => {
  expect(dashboardTabNamed(null)).toBe("command-central");
  expect(dashboardTabNamed("tiles")).toBe("command-central");
  expect(dashboardTabNamed("done")).toBe("done");
});

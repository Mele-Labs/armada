import { expect, test } from "vitest";

import { MOCK_SETTINGS } from "./fake-settings";
import { groupsFor, sectionsFor, settingsIn } from "./sections";

test("a Json setting is not drawn, so the Workflows section it alone fills is not offered", () => {
  const sections = sectionsFor(MOCK_SETTINGS).map((one) => one.id);
  expect(MOCK_SETTINGS.settings.some((one) => one.key === "workflows.off" && one.section === "Workflows")).toBe(true);
  expect(sections).not.toContain("workflows");
  expect(sections).toContain("features");
});

test("workflows.off is in no section's form, and the Features group still holds Workflows for Jobs", () => {
  const features = settingsIn(MOCK_SETTINGS, "features").map((one) => one.key);
  expect(features).toContain("features.workflows");
  expect(features).not.toContain("workflows.off");
  const group = groupsFor(MOCK_SETTINGS).find((one) => one.id === "features");
  expect(group?.sections.map((one) => one.id)).toEqual(["features"]);
});

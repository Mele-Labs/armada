import { expect, test } from "vitest";
import type { SettingsList } from "@armada/protocol";

import { spacingOf } from "./spacing";

const list = (value: unknown) => ({ settings: [{ key: "spacing.density", value }] }) as unknown as SettingsList;

test("Compact and Roomy are read as Fleet says them", () => {
  expect(spacingOf(list("compact"))).toBe("compact");
  expect(spacingOf(list("roomy"))).toBe("roomy");
});

test("unread, absent or any other value is Default", () => {
  expect(spacingOf(null)).toBe("default");
  expect(spacingOf(undefined)).toBe("default");
  expect(spacingOf({ settings: [] } as unknown as SettingsList)).toBe("default");
  expect(spacingOf(list("default"))).toBe("default");
  expect(spacingOf(list("spacious"))).toBe("default");
});

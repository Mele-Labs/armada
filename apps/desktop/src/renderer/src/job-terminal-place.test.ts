import { expect, test } from "vitest";

import type { SettingsList } from "@armada/protocol";
import { jobTerminalPlaceOf } from "./job-terminal-place";

const listing = (...rows: { key: string; value: unknown }[]) => ({ settings: rows }) as unknown as SettingsList;
const KEY = "terminal.openJobTerminalIn";

test("a Job's terminal opens in the side panel or a window only where the setting says so", () => {
  expect(jobTerminalPlaceOf(null)).toBe("docked");
  expect(jobTerminalPlaceOf(listing({ key: "terminal.app", value: "side panel" }))).toBe("docked");
  expect(jobTerminalPlaceOf(listing({ key: KEY, value: "docked" }))).toBe("docked");
  expect(jobTerminalPlaceOf(listing({ key: KEY, value: "side panel" }))).toBe("side panel");
  expect(jobTerminalPlaceOf(listing({ key: KEY, value: "window" }))).toBe("window");
  expect(jobTerminalPlaceOf(listing({ key: "terminal.app", value: "window" }))).toBe("docked");
  expect(jobTerminalPlaceOf(listing({ key: KEY, value: "Window" }))).toBe("docked");
  expect(jobTerminalPlaceOf(listing({ key: KEY, value: true }))).toBe("docked");
});

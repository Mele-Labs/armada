import { expect, test } from "vitest";

import type { SettingsList } from "@armada/protocol";
import { pullRequestPlaceOf } from "./pull-request-place";

const listing = (...rows: { key: string; value: unknown }[]) => ({ settings: rows }) as unknown as SettingsList;
const KEY = "links.openPullRequestsIn";

test("a pull request opens in the browser only where the setting says browser", () => {
  expect(pullRequestPlaceOf(null)).toBe("armada");
  expect(pullRequestPlaceOf(listing({ key: "editor.command", value: "code" }))).toBe("armada");
  expect(pullRequestPlaceOf(listing({ key: KEY, value: "armada" }))).toBe("armada");
  expect(pullRequestPlaceOf(listing({ key: KEY, value: "browser" }))).toBe("browser");
  expect(pullRequestPlaceOf(listing({ key: KEY, value: true }))).toBe("armada");
});

// The native app's Job browser runs Classic's page finder from a checked-in build, so an agent reads one snapshot shape
// in either app. This fails while that copy is older than the finder it was built from.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { built, OUT } from "../../scripts/job-browser-page.mjs";

describe("the native Job browser's page finder", () => {
  it("is the build of Classic's finder as it stands", async () => {
    expect(
      readFileSync(OUT, "utf8") === (await built()),
      "Run `pnpm --filter @armada/desktop job-browser-page` and commit what it writes.",
    ).toBe(true);
  }, 60_000);
});

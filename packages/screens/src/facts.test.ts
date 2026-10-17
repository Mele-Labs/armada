import { describe, expect, it } from "vitest";

import { money, settledBadgeOf } from "./facts";

describe("what a settled pull request reads as", () => {
  it("tells a pull request that was turned down from one that landed", () => {
    expect(settledBadgeOf("merged")?.label).toBe("Merged");
    expect(settledBadgeOf("closed_unmerged")?.label).toBe("Closed without merging");
    expect(settledBadgeOf("merged")?.status).not.toBe(settledBadgeOf("closed_unmerged")?.status);
  });

  it("draws nothing for a pull request nobody has settled", () => {
    expect(settledBadgeOf(undefined)).toBeUndefined();
    expect(settledBadgeOf("open")).toBeUndefined();
  });
});

describe("a spend", () => {
  it("rounds to cents, and to four places under a cent so it does not read as free", () => {
    expect(money(5_280_000)).toBe("~$5.28");
    expect(money(1_000)).toBe("~$0.0010");
    expect(money(0)).toBe("~$0.00");
  });
});

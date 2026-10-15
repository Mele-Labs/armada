import { describe, expect, it } from "vitest";

import { plainOf } from "@armada/components";

describe("plainOf", () => {
  it("drops colours and titles and a line break's carriage return, and keeps the text", () => {
    expect(plainOf("\u001b[1;32mok\u001b[0m\r\n\u001b]0;title\u0007next\r\n")).toBe("ok\nnext\n");
  });

  it("keeps the last drawing of a line a program redrew in place", () => {
    expect(plainOf("10%\r20%\r\u001b[K100%\r\ndone")).toBe("100%\ndone");
  });
});

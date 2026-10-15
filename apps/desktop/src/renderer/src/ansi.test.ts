import { describe, expect, it } from "vitest";

import { ansiStripper } from "./ansi";

describe("ansiStripper", () => {
  it("drops colours, titles and carriage returns and keeps the text", () => {
    const strip = ansiStripper();
    expect(strip("\u001b[1;32mok\u001b[0m\r\n\u001b]0;title\u0007next\r\n")).toBe("ok\nnext\n");
  });

  it("holds a sequence that a chunk ends inside for the next one", () => {
    const strip = ansiStripper();
    expect(strip("a\u001b[3")).toBe("a");
    expect(strip("1mb")).toBe("b");
    expect(strip("c\u001b")).toBe("c");
    expect(strip("]0;ti")).toBe("");
    expect(strip("tle\u0007d")).toBe("d");
  });
});

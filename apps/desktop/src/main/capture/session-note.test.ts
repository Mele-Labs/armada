import { describe, expect, it } from "vitest";

import type { StudioCapture } from "@armada/protocol";
import { sessionNoteText } from "./session-note";

const capture = {
  component: "Clock",
  selector: "main > section > button.clock",
  element: { tag: "button", text: "Now" },
  location: "top right",
  bounds: { x: 1, y: 2, width: 3, height: 4 },
  window: { width: 100, height: 100 },
  markup: "<button>Now</button>",
} satisfies StudioCapture;

describe("a note taken on a page a Session showed", () => {
  it("says the person's words, then the element and the address, and nothing of the markup", () => {
    const text = sessionNoteText("  Too slow  ", capture, "http://localhost:5173/");
    expect(text).toBe("Too slow\nOn Clock · … > section > button.clock, top right, at http://localhost:5173/");
    expect(text).not.toContain("<button>");
  });
});

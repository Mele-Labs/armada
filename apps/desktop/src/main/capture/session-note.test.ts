import { describe, expect, it } from "vitest";

import type { StudioCapture } from "@armada/protocol";
import { approvedText, reviewNoteOf } from "./session-note";

const capture = {
  component: "Clock",
  selector: "main > section > button.clock",
  element: { tag: "button", text: "Now" },
  location: "top right",
  bounds: { x: 1, y: 2, width: 3, height: 4 },
  window: { width: 100, height: 100 },
  markup: "<button>Now</button>",
} satisfies StudioCapture;

describe("an Approve on one of several takes", () => {
  const take = { letter: "B", name: "Needs you first" };

  it("names the take on screen, and says what it did before when there is one page", () => {
    expect(approvedText("http://localhost:5173/?b", take)).toBe("Approved B, Needs you first: http://localhost:5173/?b");
    expect(approvedText("http://localhost:5173/")).toBe("Approved: http://localhost:5173/");
  });
});

describe("a note taken in a pull request's walk", () => {
  const walk = { walkId: "w1", manifestId: "armada", number: 12, title: "#12 Take a draft out", address: "http://localhost:5173" };

  it("stays in Armada where no Session holds the pull request, as the person's words and the place", () => {
    const note = reviewNoteOf(walk, " Too slow ", capture, "2026-10-16T10:00:00Z");
    expect(note).toEqual({ manifestId: "armada", number: 12, at: "2026-10-16T10:00:00Z", text: "Too slow", location: "Clock · … > section > button.clock, top right" });
  });
});

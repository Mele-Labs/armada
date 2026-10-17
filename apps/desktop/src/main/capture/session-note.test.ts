import { describe, expect, it } from "vitest";

import type { StudioCapture } from "@armada/protocol";
import { approvedText, reviewNoteOf, sessionNoteText } from "./session-note";

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

describe("a note and an Approve on one of several takes", () => {
  const take = { letter: "B", name: "Needs you first" };

  it("name the take on screen first, and say what they did before when there is one page", () => {
    expect(sessionNoteText("Too slow", capture, "http://localhost:5173/?b", take)).toBe(
      "Too slow\nOn B, Needs you first: Clock · … > section > button.clock, top right, at http://localhost:5173/?b",
    );
    expect(approvedText("http://localhost:5173/?b", take)).toBe("Approved B, Needs you first: http://localhost:5173/?b");
    expect(approvedText("http://localhost:5173/")).toBe("Approved: http://localhost:5173/");
  });
});

describe("a note taken in a pull request's walk", () => {
  const walk = { walkId: "w1", manifestId: "armada", number: 12, title: "#12 Take a draft out", address: "http://localhost:5173" };

  it("is the holding Session's message, naming the pull request and the place", () => {
    const note = reviewNoteOf({ ...walk, holder: { id: "s1", title: "Drafts" } }, " Too slow ", capture, "2026-10-16T10:00:00Z");
    expect(note).toEqual({
      toSession: { id: "s1", text: "Walking #12 Take a draft out\nToo slow\nOn Clock · … > section > button.clock, top right, at http://localhost:5173" },
    });
  });

  it("stays in Armada where no Session holds the pull request, as the person's words and the place", () => {
    const note = reviewNoteOf(walk, " Too slow ", capture, "2026-10-16T10:00:00Z");
    expect(note).toEqual({
      kept: { manifestId: "armada", number: 12, at: "2026-10-16T10:00:00Z", text: "Too slow", location: "Clock · … > section > button.clock, top right" },
    });
  });
});

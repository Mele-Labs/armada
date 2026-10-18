// The body a note in a Session's window becomes, `send_session_note`'s, from what the window held.

import { describe, expect, it } from "vitest";

import type { StudioCapture } from "@armada/protocol";

import type { CaptureWalk } from "../../shared/capture-window";
import { sessionNoteBody, type SessionNoteHeld } from "./pinned-note";

const capture: StudioCapture = {
  component: "ShareRows",
  owners: ["UsagePage", "Boundary"],
  selector: 'ul[aria-label="Burn by Bash command"] > li[aria-label="Other commands, 16.1% of the burn"]',
  element: { tag: "li", text: "Other commands16.1%", label: "Other commands, 16.1% of the burn" },
  screen: "Usage",
  location: "/?walk=burnBySessionModelTool&scenario=usage",
  bounds: { x: 0, y: 0, width: 300, height: 24 },
  window: { width: 1200, height: 800 },
  styles: { color: "red" },
  markup: "<li></li>",
};

const walk: CaptureWalk = { name: "burnBySessionModelTool", at: 6, of: 9, said: "The Bash rows", autoplay: false, ready: true, done: false };

const held = (over: Partial<SessionNoteHeld> = {}): SessionNoteHeld => ({
  sessionId: "s-1",
  said: "  This row should name the command  ",
  capture,
  read: { stamp: "packages/components/src/compositions/UsagePage/UsagePage.tsx:171", nearby: ["Burn by Bash command", "Usage"], selected: ['tab "Bash"'] },
  address: "http://localhost:5173/?walk=burnBySessionModelTool",
  take: { letter: "B", name: "Needs you first" },
  walk,
  crop: "iVBORw0KGgo=",
  ...over,
});

describe("a Session's note, whole", () => {
  it("carries the place, the state, the walk step, the take and the crop", () => {
    expect(sessionNoteBody(held())).toEqual({
      session_id: "s-1",
      said: "This row should name the command",
      address: "http://localhost:5173/?walk=burnBySessionModelTool",
      place: {
        component: "ShareRows",
        owners: ["UsagePage", "Boundary"],
        selector: capture.selector,
        element: { tag: "li", text: "Other commands16.1%", label: "Other commands, 16.1% of the burn" },
        nearby: ["Burn by Bash command", "Usage"],
        stamp: "packages/components/src/compositions/UsagePage/UsagePage.tsx:171",
        location: "/?walk=burnBySessionModelTool&scenario=usage",
        screen: "Usage",
      },
      state: {
        selected: ['tab "Bash"'],
        walk: { name: "burnBySessionModelTool", step: 6, of: 9 },
        take: { letter: "B", name: "Needs you first" },
        scenario: "usage",
      },
      crop: { media_type: "image/png", data: "iVBORw0KGgo=" },
    });
  });

  it("leaves out what was not there: no stamp, walk, take, scenario or crop, and no markup or styles ever", () => {
    const body = sessionNoteBody(
      held({ read: { nearby: [], selected: [] }, take: undefined, walk: null, crop: null, capture: { ...capture, location: "/", component: undefined, owners: undefined } }),
    );
    expect(body.place).not.toHaveProperty("stamp");
    expect(body.place).not.toHaveProperty("component");
    expect(body.place.owners).toEqual([]);
    expect(body.state).toEqual({ selected: [] });
    expect(body).not.toHaveProperty("crop");
    expect(body).not.toHaveProperty("note_id");
    expect(JSON.stringify(body)).not.toContain("markup");
    expect(JSON.stringify(body)).not.toContain("styles");
  });

  it("cuts the element's text to 200 characters", () => {
    const long = sessionNoteBody(held({ capture: { ...capture, element: { tag: "p", text: "x".repeat(300) } } }));
    expect(long.place.element.text).toHaveLength(200);
    expect(long.place.element).not.toHaveProperty("label");
  });
});

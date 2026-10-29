// The body one of Bridge's own ⌥⌘A pins becomes, `send_session_note`'s, with its file's id.

import { describe, expect, it } from "vitest";

import type { Annotation } from "./annotations";
import { annotationNote } from "./pinned-note";

const note: Annotation = {
  id: "20261017-222105-k3f9",
  status: "open",
  text: "Too quiet ",
  component: "NeedsBand",
  source: { file: "packages/components/src/compositions/Cockpit/NeedsBand.tsx", line: 42 },
  owners: ["Cockpit"],
  ownersFrom: "parent",
  selector: 'section[aria-label="Needs you"] > ul > li:nth-of-type(2)',
  element: { tag: "li", text: "Session 3 waits on a walk", label: null },
  screen: "Cockpit",
  layer: null,
  nearby: ["Needs you"],
  location: "/index.html?scenario=every-state",
  scenario: "every-state",
  selected: ['tab "Cockpit"'],
  box: { x: 12, y: 80, width: 900, height: 64 },
  window: { width: 1280, height: 800 },
  createdAt: "2026-10-17T22:21:05.000Z",
  updatedAt: "2026-10-17T22:21:05.000Z",
};

describe("a ⌥⌘A note, whole", () => {
  it("names its file, its stamp from the build, and what the page showed", () => {
    expect(annotationNote(note, "http://localhost:5174/index.html?scenario=every-state", "iVBORw0KGgo=")).toEqual({
      said: "Too quiet",
      address: "http://localhost:5174/index.html?scenario=every-state",
      note_id: "20261017-222105-k3f9",
      place: {
        component: "NeedsBand",
        owners: ["Cockpit"],
        selector: note.selector,
        element: { tag: "li", text: "Session 3 waits on a walk" },
        nearby: ["Needs you"],
        stamp: "packages/components/src/compositions/Cockpit/NeedsBand.tsx:42",
        location: "/index.html?scenario=every-state",
        screen: "Cockpit",
      },
      state: { selected: ['tab "Cockpit"'], scenario: "every-state" },
      crop: { media_type: "image/png", data: "iVBORw0KGgo=" },
    });
  });

  it("reads a note written before nearby and selected were, with empty lists and no crop", () => {
    const { nearby: _nearby, selected: _selected, source: _source, ...older } = note;
    const body = annotationNote({ ...older, component: null, scenario: null, screen: null }, "app://bridge", null);
    expect(body.place.nearby).toEqual([]);
    expect(body.state).toEqual({ selected: [] });
    expect(body.place).not.toHaveProperty("stamp");
    expect(body.place).not.toHaveProperty("component");
    expect(body).not.toHaveProperty("crop");
  });
});

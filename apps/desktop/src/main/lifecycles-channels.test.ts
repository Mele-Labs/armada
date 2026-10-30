// What the renderer may ask main to save or delete of a Workspace, lifecycle or rule is checked for its shape before Fleet is asked. What the piece says is Fleet's parser's to refuse.

import { describe, expect, it } from "vitest";

import { deleteOf, saveOf } from "./lifecycles-channels";

const lifecycle = { kind: "lifecycle", lifecycle: { id: "ticket", name: "Ticket", scope: { kind: "everywhere" }, states: [], transitions: [], ends: [] } };

describe("a piece to save", () => {
  it("is one of the four kinds, with overwrite only as a flag", () => {
    expect(saveOf({ piece: lifecycle })).toEqual({ piece: lifecycle });
    expect(saveOf({ piece: lifecycle, overwrite: true })).toEqual({ piece: lifecycle, overwrite: true });
    expect(saveOf({ piece: lifecycle, overwrite: "yes" })).toBeNull();
    expect(saveOf({ piece: { kind: "theme" } })).toBeNull();
    expect(saveOf({ piece: "ticket" })).toBeNull();
    expect(saveOf({})).toBeNull();
    expect(saveOf(null)).toBeNull();
  });
});

describe("a piece to delete", () => {
  it("names its kind and id, and the scope its file declares", () => {
    expect(deleteOf({ kind: "rule", id: "progress" })).toEqual({ kind: "rule", id: "progress" });
    expect(deleteOf({ kind: "lifecycle", id: "ticket", scope: { kind: "within", id: "01MF" } })).toEqual({ kind: "lifecycle", id: "ticket", scope: { kind: "within", id: "01MF" } });
    expect(deleteOf({ kind: "lifecycle", id: "ticket", scope: { kind: "within" } })).toBeNull();
    expect(deleteOf({ kind: "lifecycle", id: " " })).toBeNull();
    expect(deleteOf({ kind: "tile", id: "ticket" })).toBeNull();
    expect(deleteOf(undefined)).toBeNull();
  });
});

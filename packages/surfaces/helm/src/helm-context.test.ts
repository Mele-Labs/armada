// Helm's context, on its own — no window, no Fleet.

import { expect, test } from "vitest";
import { contextOf, locationOf, screenOf } from "./helm-context";

test("screenOf follows the precedence App.tsx draws by", () => {
  const none = {
    clearing: false,
    manifesting: false,
    studying: false,
    kitting: false,
    settling: false,
  };
  const every = {
    clearing: true,
    manifesting: true,
    studying: true,
    kitting: true,
    settling: true,
  };
  expect(screenOf(every)).toBe("cleanup");
  expect(screenOf({ ...every, clearing: false })).toBe("manifest");
  expect(screenOf({ ...none, studying: true, kitting: true, settling: true })).toBe("studio");
  expect(screenOf({ ...none, kitting: true, settling: true })).toBe("kit");
  // Overview, not the Board: everything with nothing else open is Overview
  // with something over it, which is what that page's deletion left behind.
  expect(screenOf(none)).toBe("overview");
});

/**
 * **Settings was missing from this enum until #1275**, so a person on it was
 * reported to Helm as being on the Board — the gap #1287 left when it added
 * `studio` and stopped. Its own case, because the one above proves precedence
 * and this proves the value exists at all.
 */
test("screenOf names Settings rather than falling through to Overview", () => {
  const none = {
    clearing: false,
    manifesting: false,
    studying: false,
    kitting: false,
    settling: false,
  };
  expect(screenOf({ ...none, settling: true })).toBe("settings");
});

test("screenOf names Workflows, and Kit still outranks it", () => {
  const none = {
    clearing: false,
    manifesting: false,
    studying: false,
    kitting: false,
    settling: false,
  };
  expect(screenOf({ ...none, workflowing: true })).toBe("workflows");
  expect(screenOf({ ...none, workflowing: true, kitting: true })).toBe("kit");
  expect(locationOf({ screen: "workflows" })).toBe("Workflows");
});

test("contextOf leaves an absent field off the wire", () => {
  expect(contextOf({ screen: "overview", picked: null })).toEqual({ screen: "overview" });
  expect(contextOf({ screen: "overview", picked: "M-armada" })).toEqual({ screen: "overview", picked: "M-armada" });
});

test("locationOf names the screen alone", () => {
  expect(locationOf({ screen: "overview" })).toBe("Cockpit");
});

test("a Studio rides on the context only on the Studios surface, and a node only beside its Studio", () => {
  const base = { picked: null };
  expect(contextOf({ ...base, screen: "studio", studio: "s1", node: "n1" })).toEqual({ screen: "studio", studio: "s1", node: "n1" });
  expect(contextOf({ ...base, screen: "studio", studio: null, node: "n1" })).toEqual({ screen: "studio" });
  expect(contextOf({ ...base, screen: "overview", studio: "s1", node: "n1" })).toEqual({ screen: "overview" });
});

test("locationOf names the Studio open, and the node selected on it", () => {
  expect(locationOf({ screen: "studio" })).toBe("Studios");
  expect(locationOf({ screen: "studio", studio: "s1" }, { name: "Untitled Studio" })).toBe("Studios · Untitled Studio");
  expect(locationOf({ screen: "studio", studio: "s1", node: "n1" }, { name: "Legend", node: "Note The legend" })).toBe(
    "Studios · Legend · Note The legend selected",
  );
});

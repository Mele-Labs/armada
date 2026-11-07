import { readKept, writeKept } from "@armada/components";
import { beforeEach, expect, test, vi } from "vitest";

import {
  customDraftKey,
  customLabel,
  followJob,
  forgetCustom,
  forgetOldCustomDrafts,
  peekCustom,
  seedBuilt,
  startBuilding,
} from "./custom";

/** A window whose storage lists its keys, as the browser's does. */
beforeEach(() => {
  const store = new Map<string, string>();
  vi.stubGlobal("window", {
    localStorage: {
      get length() {
        return store.size;
      },
      key: (at: number) => [...store.keys()][at] ?? null,
      getItem: (key: string) => store.get(key) ?? null,
      setItem: (key: string, value: string) => void store.set(key, value),
      removeItem: (key: string) => void store.delete(key),
    },
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  });
  forgetCustom("a");
  forgetCustom("b");
});

test("two Jobs with the same tab id keep their own stage, mod and Job", () => {
  startBuilding("a", "t1", "mod-job");
  seedBuilt("b", "t1", "pull-queue");
  expect(peekCustom("a", "t1")).toEqual({
    stage: "building",
    job: "mod-job",
  });
  expect(peekCustom("b", "t1")).toEqual({
    stage: "built",
    mod: "pull-queue",
  });
  followJob("b", "t1", "fixer");
  expect(peekCustom("a", "t1").job).toBe("mod-job");
  expect(peekCustom("b", "t1").job).toBe("fixer");
});

test("a tab's label is its own Job's", () => {
  seedBuilt("a", "t1", "review-queue");
  expect(customLabel("a", "t1")).toBe("Review queue");
  expect(customLabel("b", "t1")).toBeUndefined();
});

test("the draft key differs per Job", () => {
  expect(customDraftKey("a", "t1")).toBe("custom:a:t1");
  expect(customDraftKey("a", "t1")).not.toBe(customDraftKey("b", "t1"));
});

test("forgetting a Job drops its tabs and drafts and leaves another's", () => {
  seedBuilt("a", "t1", "one");
  seedBuilt("b", "t1", "two");
  writeKept(`session:${customDraftKey("a", "t1")}`, "mine");
  writeKept(`session:${customDraftKey("b", "t1")}`, "theirs");
  forgetCustom("a");
  expect(peekCustom("a", "t1").stage).toBe("compose");
  expect(readKept(`session:${customDraftKey("a", "t1")}`)).toBeUndefined();
  expect(peekCustom("b", "t1").mod).toBe("two");
  expect(readKept(`session:${customDraftKey("b", "t1")}`)).toBe("theirs");
});

test("old-shape drafts go and new-shape ones stay", () => {
  writeKept("session:custom:t1", "old");
  writeKept(`session:${customDraftKey("a", "t1")}`, "new");
  writeKept("session:other:t1", "other");
  forgetOldCustomDrafts();
  expect(readKept("session:custom:t1")).toBeUndefined();
  expect(readKept(`session:${customDraftKey("a", "t1")}`)).toBe("new");
  expect(readKept("session:other:t1")).toBe("other");
});

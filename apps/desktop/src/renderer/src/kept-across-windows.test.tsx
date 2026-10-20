// A draft kept in one Bridge window and sent from another — a walk's Job panel
// shares this window's origin and storage, and fires `storage` here when it writes.

import { beforeEach, expect, test } from "vitest";
import { forgetAllKept, readKept, writeKept } from "@armada/components";

const STORED = "armada:kept:session:x";

beforeEach(forgetAllKept);

const otherWindowWrote = () => window.dispatchEvent(new StorageEvent("storage", { key: STORED }));

test("a draft another window sent is gone here", () => {
  writeKept("session:x", { text: "hi" });
  window.localStorage.removeItem(STORED);
  otherWindowWrote();
  expect(readKept("session:x")).toBeUndefined();
});

test("a draft another window typed is read here", () => {
  writeKept("session:x", { text: "hi" });
  window.localStorage.setItem(STORED, JSON.stringify({ text: "typed over there" }));
  otherWindowWrote();
  expect(readKept("session:x")).toEqual({ text: "typed over there" });
});

// The first message an agent wrote for a Session it started is put in that Session's box once, and only
// where the person has not already kept words there. Once he sends it Fleet drops it, and nothing reseeds.

import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { forgetAllKept, forgetKept, readKept, writeKept } from "@armada/components";
import type { WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { isUnsentItem, unsentEntry, useStartedBy } from "./session-started";

const roots: { root: Root; host: HTMLElement }[] = [];

beforeEach(forgetAllKept);
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  forgetAllKept();
});

const session = (extra: Record<string, unknown> = {}): Session => ({ id: "new1", attachments: [], rows: [], turn: { state: "idle" }, ...extra }) as unknown as Session;

function Probe({ one }: { one: Session }) {
  return <>{useStartedBy(one, [one], () => {})}</>;
}

function mount(one: Session): (next: Session) => void {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  flushSync(() => root.render(<Probe one={one} />));
  return (next) => flushSync(() => root.render(<Probe one={next} />));
}

const kept = () => readKept<{ text: string }>("session:new1")?.text;

test("puts the unsent message in an empty box", () => {
  mount(session({ unsent: "Port the clock" }));
  expect(kept()).toBe("Port the clock");
});

test("does not write over words already kept in the box", () => {
  writeKept("session:new1", { text: "mine", tags: [], files: [] });
  mount(session({ unsent: "Port the clock" }));
  expect(kept()).toBe("mine");
});

test("seeds nothing where no message is unsent", () => {
  mount(session());
  expect(readKept("session:new1")).toBeUndefined();
});

test("does not seed again once the message is sent and Fleet has dropped it", () => {
  const show = mount(session({ unsent: "Port the clock" }));
  forgetKept("session:new1");
  show(session());
  expect(readKept("session:new1")).toBeUndefined();
});

test("the item that waits on the first message is told by its source, and a press on it goes to the box", () => {
  const item = (source: WaitingItem["source"]): WaitingItem => ({ id: "unsent:new1", text: "First message", since: "2026-10-12T10:00:00Z", source });
  expect(isUnsentItem(item("first_message"))).toBe(true);
  expect(isUnsentItem(item("agent"))).toBe(false);
  let went = 0;
  unsentEntry(item("first_message"), () => (went += 1)).onOpen?.();
  expect(went).toBe(1);
});

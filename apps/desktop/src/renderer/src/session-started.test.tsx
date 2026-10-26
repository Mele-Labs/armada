// The first message of a held Job is put in its box once, and only where the person has not already kept words there.
// Once he presses Start the card is gone, and nothing reseeds.

import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test } from "vitest";
import { forgetAllKept, forgetKept, readKept, writeKept } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useStartedBy } from "./session-started";

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

test("puts a held Job's first message in an empty box", () => {
  mount(session({ start: { asksDelivery: true, message: "Port the clock" } }));
  expect(kept()).toBe("Port the clock");
});

test("does not write over words already kept in the box", () => {
  writeKept("session:new1", { text: "mine", tags: [], files: [] });
  mount(session({ start: { asksDelivery: true, message: "Port the clock" } }));
  expect(kept()).toBe("mine");
});

test("seeds nothing where the Job holds no first message", () => {
  mount(session());
  expect(readKept("session:new1")).toBeUndefined();
});

test("does not seed again once the Job has begun and the card is gone", () => {
  const show = mount(session({ start: { asksDelivery: true, message: "Port the clock" } }));
  forgetKept("session:new1");
  show(session());
  expect(readKept("session:new1")).toBeUndefined();
});

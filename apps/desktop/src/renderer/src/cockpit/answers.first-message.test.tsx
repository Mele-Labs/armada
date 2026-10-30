// A Session its agent started waits on the person sending the first message, and he sends it in that
// Session's own box. So its card draws no reply box and no answers, and its one act opens the Session.

import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, expect, test, vi } from "vitest";
import type { WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Hosts, Item } from "../Dashboard";
import { connected } from "../mock/moment";
import { useAnswering, type Answering } from "./answers";
import { asksAnAgent } from "./standing";
import { callsFromWaiting } from "./waiting";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const FIRST: WaitingItem = { id: "unsent:new1", text: "First message", since: "2026-10-12T10:00:00Z", source: "first_message", act: { kind: "message", target: "new1" } };
const session = { id: "new1", title: "Port the clock", attachments: [], rows: [], turn: { state: "idle" }, waitingFor: [FIRST] } as unknown as Session;

const cardOf = (one: WaitingItem): Item =>
  ({ key: `session:new1:${one.id}`, kind: "Session", title: "Port the clock", fact: one.text, hue: "ask", where: "", body: [], waiting: { sessionId: "new1", item: one }, acts: () => null }) as unknown as Item;

function answering(item: Item, hosts: Hosts): { current: Answering | undefined } {
  const seen: { current: Answering | undefined } = { current: undefined };
  function Probe() {
    seen.current = useAnswering(item, hosts, connected([], []), () => {});
    return null;
  }
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  act(() => root.render(<Probe />));
  return seen;
}

test("an idle Session holding an unsent first message is on the list", () => {
  expect(callsFromWaiting([session]).map((one) => one.item.id)).toEqual(["unsent:new1"]);
});

test("the first message has no standing answers", () => {
  expect(asksAnAgent(cardOf(FIRST))).toBe(false);
});

test("its card offers no answer and no reply box, and Enter opens the Session", () => {
  const hosts = { onOpenSession: vi.fn(), onOpenLink: vi.fn() };
  const card = answering(cardOf(FIRST), hosts).current!;
  expect(card.answers).toEqual([]);
  expect(card.reply).toBeUndefined();
  expect(card.canSend).toBe(true);
  card.send();
  expect(hosts.onOpenSession).toHaveBeenCalledExactlyOnceWith("new1");
});

test("a plain line from the agent, with no act, still takes a reply", () => {
  const plain: WaitingItem = { id: "a1", text: "Anything else?", since: FIRST.since, source: "agent" };
  const card = answering(cardOf(plain), { onOpenSession: vi.fn(), onOpenLink: vi.fn() }).current!;
  expect(card.reply).toBeDefined();
});

test("a sketch waiting on him offers one tap, Open the Job, which opens that Session", () => {
  const sketch: WaitingItem = { id: "sketch:Store clock", text: "Store clock", since: FIRST.since, source: "sketch", act: { kind: "message", target: "new1" } };
  const hosts = { onOpenSession: vi.fn(), onOpenLink: vi.fn() };
  const card = answering(cardOf(sketch), hosts).current!;
  expect(card.answers.map((one) => one.label)).toEqual(["Open the Job"]);
  card.answers[0]!.run();
  expect(hosts.onOpenSession).toHaveBeenCalledExactlyOnceWith("new1");
});

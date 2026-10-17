// The deck's Activity on its own: a dot for each event in the last hour, `keep` and `of` narrowing them, a press picking the dot's
// event, and `lit` ringing one. Over a source the test hands in, with no `App` and no mock Fleet.

import type { TimelineEvent } from "@armada/protocol";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { userEvent, page } from "vitest/browser";

import "../styles/index.css";
import { ActivityGraph } from "./ActivityChart";
import { FleetEventsOver, type FleetEvent } from "./fleet-events";

const at = (secondsAgo: number) => new Date(Date.now() - secondsAgo * 1000).toISOString();
const entry = (id: number, session: string, says: string, secondsAgo = 60): TimelineEvent => ({
  id,
  at: at(secondsAgo),
  kind: "session.step",
  subject: { kind: "session", id: session },
  title: `Session ${session}`,
  tone: "run",
  says,
  live: true,
});

const HELD: readonly TimelineEvent[] = [entry(1, "a", "Scope passed", 600), entry(2, "b", "Implement started", 60)];

let off: (() => void) | undefined;
afterEach(() => off?.());

function chart(props: Partial<Parameters<typeof ActivityGraph>[0]> = {}, held: readonly TimelineEvent[] = HELD) {
  const host = document.createElement("div");
  host.style.cssText = "width: 274px; height: 240px; display: flex;";
  document.body.append(host);
  const root = createRoot(host);
  off = () => {
    root.unmount();
    host.remove();
  };
  root.render(
    <FleetEventsOver source={{ get: () => held, subscribe: () => () => undefined }}>
      <ActivityGraph now={Date.now()} filter="running" onPick={() => {}} {...props} />
    </FleetEventsOver>,
  );
}

const names = () => [...document.querySelectorAll(".armada-tl__dot")].map((one) => one.getAttribute("aria-label")).sort();

test("keep narrows the dots to the subjects it keeps, and absent keeps every one", async () => {
  chart();
  await expect.poll(names).toEqual(["Session a: Scope passed", "Session b: Implement started"]);
  off?.();

  chart({ keep: (subject) => subject.kind === "session" && subject.id === "a" });
  await expect.poll(names).toEqual(["Session a: Scope passed"]);
});

test("pressing a dot picks its event, and lit rings that dot alone", async () => {
  const picked = vi.fn<(event: FleetEvent) => void>();
  chart({ onPick: picked, lit: "2" });
  await expect.poll(names).toHaveLength(2);

  expect(document.querySelector('[aria-label="Session b: Implement started"]')!.hasAttribute("data-lit")).toBe(true);
  expect(document.querySelector('[aria-label="Session a: Scope passed"]')!.hasAttribute("data-lit")).toBe(false);

  await userEvent.click(page.getByRole("button", { name: "Session a: Scope passed" }));
  expect(picked).toHaveBeenCalledTimes(1);
  expect(picked.mock.calls[0]![0]).toMatchObject({ id: "1", subject: { kind: "session", id: "a" } });
});

test("only the last hour is drawn", async () => {
  chart({}, [entry(1, "a", "Old", 3 * 3600), entry(2, "b", "New", 30)]);
  await expect.poll(names).toEqual(["Session b: New"]);
});

// A busy two hours holds over a thousand events, and a button for each froze the deck.
test("a Session opened on the deck narrows the dots to its own events however many newer ones there are", async () => {
  const many: readonly TimelineEvent[] = [
    ...Array.from({ length: 5 }, (_, i) => entry(1000 + i, "b", `B step ${i}`, 3000 - i)),
    ...Array.from({ length: 700 }, (_, i) => entry(i + 1, "a", `A step ${i + 1}`, 600 - i * 0.5)),
  ];
  chart({ of: { kind: "session", id: "b" } }, many);
  await expect.poll(() => names().length).toBe(5);
  expect(names().every((name) => name!.startsWith("Session b:"))).toBe(true);
});

// A thread keeps its end in view when it grows while opening, and when something shortens it, but not once he has moved.

import { SessionThread, type SessionThreadRow } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const rows: SessionThreadRow[] = Array.from(
  {
    length: 30,
  },
  (_, at) => ({
    id: `r${at}`,
    at: "09:00:00",
    kind: "message",
    from: "agent",
    text: `Row ${at}`,
  }),
);

const px = (length: number): string => `${length}px`;

async function opened() {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  root.render(
    <SessionThread
      sessionId="s"
      rows={rows}
      onAnswer={() => undefined}
      onOpenSession={() => undefined}
    />,
  );
  const thread = await vi.waitFor(() => {
    const found = host.querySelector<HTMLElement>(".armada-session-thread__rows");
    expect(found).not.toBeNull();
    return found!;
  });
  // The frame's height, which a test has none of.
  thread.style.height = px(400);
  thread.style.overflowY = "auto";
  return {
    thread,
    gap: () => Math.round(thread.scrollHeight - thread.scrollTop - thread.clientHeight),
    // Below what he reads, so the browser's scroll anchoring does not keep the end for it.
    lastRow: [
      ...thread.querySelectorAll<HTMLElement>("li"),
    ].at(-1)!,
  };
}

const frames = () =>
  new Promise<void>((done) => {
    requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(done)));
  });

test("stays at its end when its text grows as it opens", async () => {
  const { gap, lastRow } = await opened();
  await vi.waitFor(() => expect(gap()).toBe(0));
  lastRow.style.paddingBottom = px(240);
  await frames();
  expect(gap()).toBe(0);
});

test("stays at its end when the screen shortens it", async () => {
  const { thread, gap } = await opened();
  await vi.waitFor(() => expect(gap()).toBe(0));
  thread.style.height = px(250);
  await frames();
  expect(gap()).toBe(0);
  // Still following afterwards, which a shortened box reading as him leaving would stop.
  thread.style.height = px(200);
  await frames();
  expect(gap()).toBe(0);
});

test("leaves what he opened where he reads it, once he has pressed", async () => {
  const { thread, gap, lastRow } = await opened();
  await vi.waitFor(() => expect(gap()).toBe(0));
  thread.dispatchEvent(
    new PointerEvent("pointerdown", {
      bubbles: true,
    }),
  );
  lastRow.style.paddingBottom = px(240);
  await frames();
  expect(gap()).toBeGreaterThan(200);
});

test("does not bring him back once he has scrolled up", async () => {
  const { thread, gap } = await opened();
  await vi.waitFor(() => expect(gap()).toBe(0));
  thread.scrollTop = 0;
  await frames();
  thread.dispatchEvent(new Event("scroll"));
  thread.style.height = px(250);
  await frames();
  expect(thread.scrollTop).toBe(0);
});

// The Judge tile's states, read from the Session's `judging` and `judged`: working while a judge reads, "No concerns" when a reading
// with none landed after the press, and nothing otherwise. Browser: a press and a rendering are what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { SessionTiles } from "./SessionTiles";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
beforeEach(() => vi.useRealTimers());
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const session = (work: Session["work"]): Session => ({ id: "s1", rows: [], attachments: [], turn: { state: "idle" }, work: { plan: { scope: [], groups: [], approved: false }, ...work } }) as unknown as Session;
const draftOf = (judge: () => Promise<void>) => ({ commands: [], get: () => [], work: { judge } }) as unknown as SessionsDraft;

function mount(one: Session, draft: SessionsDraft) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const draw = (next: Session) => act(() => root.render(<SessionTiles session={next} draft={draft} repository={undefined} />));
  draw(one);
  roots.push({ root, host });
  return { host, draw };
}

const tile = (host: HTMLElement) => host.querySelector<HTMLElement>('[aria-label="Judge"]')!;
const mark = (host: HTMLElement, name: string) => host.querySelector(`[role="img"][aria-label="${name}"]`);

test("the tile is plain until a judge is sent", () => {
  const { host } = mount(session({}), draftOf(async () => undefined));
  expect(mark(host, "Judge reading")).toBeNull();
  expect(mark(host, "No concerns")).toBeNull();
});

test("the tile works while Fleet says a judge is reading, and stops when the record says it is done", () => {
  const { host, draw } = mount(session({ judging: true }), draftOf(async () => undefined));
  expect(mark(host, "Judge reading")).not.toBeNull();
  draw(session({}));
  expect(mark(host, "Judge reading")).toBeNull();
});

test("a press asks for a reading, and a reading with no concerns that lands after it reads No concerns", async () => {
  const judge = vi.fn(async () => undefined);
  const { host, draw } = mount(session({}), draftOf(judge));
  await act(async () => tile(host).click());
  expect(judge).toHaveBeenCalledWith("s1");
  draw(session({ judged: { at: new Date(Date.now() + 1000).toISOString(), concerns: 0 } }));
  expect(mark(host, "No concerns")).not.toBeNull();
});

test("a reading from before the press is not this press's answer, and one that found concerns is not No concerns", async () => {
  const { host, draw } = mount(session({ judged: { at: new Date(Date.now() - 60_000).toISOString(), concerns: 0 } }), draftOf(async () => undefined));
  expect(mark(host, "No concerns")).toBeNull();
  await act(async () => tile(host).click());
  expect(mark(host, "No concerns")).toBeNull();
  draw(session({ judged: { at: new Date(Date.now() + 1000).toISOString(), concerns: 2 } }));
  expect(mark(host, "No concerns")).toBeNull();
});

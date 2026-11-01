// The live provider reads a Job's quick replies from Fleet (`GET /sessions/quick`) and puts them on its bar: the answers to the
// agent's last message first, then his own lines, then the lines he sends often. The window's fake has no settings to follow,
// so the look is the default and `setLook` moves it, as a press in the Aa panel does.

import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { useThreadLook } from "@armada/components";
import type { ThreadLookSource } from "@armada/components";
import type { SessionQuick } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { afterEach, beforeEach, expect, test, vi } from "vitest";

import { ThreadLookSaved, useThreadLookHere } from "./thread-look-here";

const roots: { root: Root; host: HTMLElement }[] = [];
let read: ReturnType<typeof vi.fn>;
let answers: Record<string, SessionQuick> = {};
let source: ThreadLookSource | null = null;
let here: ReturnType<typeof useThreadLookHere>;

beforeEach(() => {
  answers = { s1: { asked: ["Yes, go on", "Not yet"], learnt: ["Just get it done", "Walk me through it"] } };
  read = vi.fn(async (id: string) => ({ ok: true, value: answers[id] ?? { asked: [], learnt: [] } }));
  (window as unknown as { armada: unknown }).armada = { readSessionQuick: read };
});

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  source = null;
  here = undefined;
  delete (window as unknown as { armada?: unknown }).armada;
});

const message = (id: string, from: "agent" | "you", text: string) => ({ kind: "message", id, at: "2026-10-31T12:00:00Z", from: { kind: from }, text });
const tool = (id: string) => ({ kind: "tool", id, at: "2026-10-31T12:00:00Z", text: "Reading a file" });
const job = (id: string, ...rows: object[]) => ({ id, rows }) as unknown as Session;

function Probe({ session }: { session: Session }) {
  source = useThreadLook();
  here = useThreadLookHere(session);
  return null;
}

function mount(session: Session) {
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  const show = (next: Session) => flushSync(() => root.render(<ThreadLookSaved><Probe session={next} /></ThreadLookSaved>));
  show(session);
  return show;
}

const bar = () => here?.composer.quick;
const asked = () => bar()?.asked ?? [];
const learnt = () => (bar()?.phrases ?? []).filter((one) => one.kind === "learnt").map((one) => one.text);

test("asks Fleet about a Job whose bar is drawn, and puts the answers and the lines he sends often on it", async () => {
  mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(asked).toEqual(["Yes, go on", "Not yet"]);
  expect(learnt()).toEqual(["Just get it done", "Walk me through it"]);
  expect(read).toHaveBeenCalledTimes(1);
  expect(read).toHaveBeenCalledWith("s1");
  expect(bar()?.phrases.every((one) => one.id === `learnt:${one.text}`)).toBe(true);
});

test("reads again when the Job's last message changes, not for the calls between, and drops the old answers at once", async () => {
  const show = mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(asked).toEqual(["Yes, go on", "Not yet"]);

  show(job("s1", message("m1", "agent", "Shall I carry on?"), tool("t1"), tool("t2")));
  expect(read).toHaveBeenCalledTimes(1);

  answers = { s1: { asked: ["Ship it"], learnt: ["Just get it done"] } };
  show(job("s1", message("m1", "agent", "Shall I carry on?"), message("m2", "agent", "Done. Ship it?")));
  expect(asked()).toEqual([]);
  await expect.poll(asked).toEqual(["Ship it"]);
  expect(read).toHaveBeenCalledTimes(2);
});

test("offers the answers only while the agent spoke last, and keeps the lines he sends often either way", async () => {
  mount(job("s1", message("m1", "agent", "Shall I carry on?"), message("m2", "you", "Yes")));
  await expect.poll(learnt).toEqual(["Just get it done", "Walk me through it"]);
  expect(asked()).toEqual([]);
});

test("asks nothing for a Job whose bar is hidden", async () => {
  const show = mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(asked).toEqual(["Yes, go on", "Not yet"]);
  flushSync(() => source!.setLook((was) => ({ ...was, quickOffFor: ["s2"] })));
  show(job("s2", message("m1", "agent", "Shall I carry on?")));
  expect(bar()).toBeUndefined();
  expect(read).not.toHaveBeenCalledWith("s2");
});

test("drops the learnt lines while he has learning off, and asks again when he turns it on", async () => {
  mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(learnt).toHaveLength(2);
  flushSync(() => source!.setLook((was) => ({ ...was, learn: false })));
  expect(learnt()).toEqual([]);
  flushSync(() => source!.setLook((was) => ({ ...was, learn: true })));
  await expect.poll(() => read.mock.calls.length).toBe(3);
  expect(learnt()).toEqual(["Just get it done", "Walk me through it"]);
});

test("leaves out a learnt line he has kept or forgotten, wherever Fleet has not yet caught up", async () => {
  mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(learnt).toHaveLength(2);
  flushSync(() => source!.setLook((was) => ({ ...was, quick: [{ id: "saved:1", text: "just get it done.", kind: "saved" }], forgotten: ["Walk me through it"] })));
  expect(learnt()).toEqual([]);
  expect(bar()?.phrases.map((one) => one.text)).toEqual(["just get it done."]);
});

test("a press on a line Fleet learnt takes hold: deleting it forgets its words", async () => {
  mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(learnt).toHaveLength(2);
  flushSync(() => bar()!.onAct!({ act: "delete", id: "learnt:Just get it done" }));
  expect(source!.look.forgotten).toEqual(["Just get it done"]);
  expect(learnt()).toEqual(["Walk me through it"]);
});

test("keeps a learnt line as his own when he pins it", async () => {
  mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(learnt).toHaveLength(2);
  flushSync(() => bar()!.onAct!({ act: "pin", id: "learnt:Walk me through it" }));
  expect(source!.look.quick).toEqual([{ id: "learnt:Walk me through it", text: "Walk me through it", kind: "saved", pinned: true }]);
  expect(learnt()).toEqual(["Just get it done"]);
});

test("a read Fleet refused leaves the bar without Fleet's lines and asks again at the next message", async () => {
  read.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "not_connected" } });
  const show = mount(job("s1", message("m1", "agent", "Shall I carry on?")));
  await expect.poll(() => read.mock.calls.length).toBe(1);
  expect(asked()).toEqual([]);
  show(job("s1", message("m1", "agent", "Shall I carry on?"), message("m2", "you", "Yes")));
  await expect.poll(learnt).toEqual(["Just get it done", "Walk me through it"]);
});

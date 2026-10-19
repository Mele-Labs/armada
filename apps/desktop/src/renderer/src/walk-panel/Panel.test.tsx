// The panel beside the walk window's page, as `main.tsx` mounts it: the Job of the walk on screen drawn as the Cockpit's
// `SessionView`, switching Job as main pushes the next walk's, and nothing where that walk has none (a pull request's).
// `docs/practices/capture-window.md`, *The panel beside the page*.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Outcome } from "@armada/protocol";
import type { PanelHeldNote } from "@armada/studios/api";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { NOTHING_YET } from "../../../shared/bridge";
import { SessionsFrom } from "../sessions-draft";
import { Panel } from "./Panel";
import { useWalkSubject } from "./subject";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const session = (id: string, title: string, more: Partial<Session> = {}): Session => ({
  id,
  title,
  attachments: [],
  rows: [{ id: `${id}-1`, at: "2026-10-19T09:00:00Z", kind: "message", from: { kind: "agent" }, text: `${title}, from the agent` }],
  turn: { state: "idle" },
  ...more,
});

const SESSIONS = [
  session("s1", "Retry the lease"),
  session("s2", "Name the slots"),
];
const draft = { models: ["opus"], efforts: ["high"], send: () => {}, tune: () => {}, answer: () => {}, start: () => "x", subscribe: () => () => {}, get: () => SESSIONS } as unknown as SessionsDraft;

/** What the panel's document does with the subject main pushes: `main.tsx`'s `Followed`. */
function Followed() {
  const id = useWalkSubject();
  return <Panel subject={id === undefined ? null : { kind: "session", id }} />;
}

let push: (sessionId: string | undefined) => void = () => {};
let tell: (notes: PanelHeldNote[]) => void = () => {};
const asked = vi.fn();
const acts = { editNote: vi.fn(async (): Promise<Outcome> => ({ ok: true })), removeNote: vi.fn(async (): Promise<Outcome> => ({ ok: true })), sendNotes: vi.fn(async (): Promise<Outcome> => ({ ok: true })) };
let root: Root;
let host: HTMLElement;

beforeEach(() => {
  (window as unknown as { armada: unknown }).armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({ full: NOTHING_YET, seq: 1 }),
    walkPanel: {
      goInBridge: async () => {},
      openInBridge: async () => {},
      subject: (onChanged: typeof push) => {
        push = onChanged;
        return () => {};
      },
      held: (onChanged: typeof tell) => {
        asked();
        tell = onChanged;
        return () => {};
      },
      ...acts,
    },
  };
  asked.mockClear();
  for (const one of Object.values(acts)) one.mockClear();
  acts.sendNotes.mockImplementation(async () => ({ ok: true }));
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
});

afterEach(() => {
  act(() => root.unmount());
  host.remove();
});

const frame = (id: string) => host.querySelector(`[role="region"][aria-label="Job ${id}"]`);

async function draw() {
  await act(async () =>
    root.render(
      <SessionsFrom held={draft}>
        <Followed />
      </SessionsFrom>,
    ),
  );
}

test("draws the Job of the walk on screen as the Cockpit's view: its frame, thread and ledger", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  expect(host.querySelector('section[aria-label="Job: Retry the lease"]')).not.toBeNull();
  expect(host.querySelector('[role="region"][aria-label="Thread"]')?.textContent).toContain("Retry the lease, from the agent");
  // What the Dashboard's miniature left out: the Job's frame round the thread, and the ledger beside it.
  expect(host.querySelector('aside[aria-label="Attachments"]')).not.toBeNull();
});

test("switches to the next walk's Job as main pushes it", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  await act(async () => push("s2"));
  await expect.poll(() => frame("s2")).not.toBeNull();
  expect(frame("s1")).toBeNull();
  expect(host.querySelector('[role="region"][aria-label="Thread"]')?.textContent).toContain("Name the slots, from the agent");
});

test("draws nothing for a walk that has no Job", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  await act(async () => push(undefined));
  await expect.poll(() => host.innerHTML).toBe("");
});

// A one-pixel PNG, as main's crop is base64 PNG.
const PIXEL = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";
const HELD: PanelHeldNote[] = [
  { id: "n1", at: "2026-10-19T14:00:00Z", said: "Too tight", on: "button “Reload”", crop: { media_type: "image/png", data: PIXEL } },
  { id: "n2", at: "2026-10-19T14:01:00Z", said: "Wrong colour", on: "heading “Backoff”" },
];
const notes = () => host.querySelector('section[aria-label="Pending notes"]');
const send = () => [...notes()!.querySelectorAll("button")].find((one) => one.textContent === "Send notes") ?? null;
const thread = () => host.querySelector('[role="region"][aria-label="Thread"]');
const note = (said: string) => host.querySelector<HTMLElement>(`li[aria-label="${said}"]`);
const press = async (element: Element | null) => act(async () => (element as HTMLElement).click());
const typed = async (field: HTMLTextAreaElement, words: string) =>
  act(async () => {
    field.focus();
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set?.call(field, words);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });

async function holding(held: PanelHeldNote[] = HELD) {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  await act(async () => tell(held));
  await expect.poll(() => notes()).not.toBeNull();
}

test("draws the Job's held notes as the last entry of its thread, each with its crop, what it is on and its words, and one Send at the foot", async () => {
  await holding();
  expect(note("Too tight")?.querySelector("img")?.getAttribute("alt")).toBe("Crop of button “Reload”");
  await expect.poll(() => note("Too tight")?.querySelector("img")?.getAttribute("src")).toMatch(/^blob:/);
  expect(note("Wrong colour")?.querySelector("img")).toBeNull();
  expect(note("Too tight")?.textContent).toContain("On button “Reload”");
  expect(note("Too tight")?.querySelector("textarea")?.value).toBe("Too tight");
  expect([...notes()!.querySelectorAll("button")].map((one) => one.textContent)).toEqual(["Remove", "Remove", "Send notes"]);
  // In the thread, after its newest row, and not above the view.
  expect(thread()?.contains(notes())).toBe(true);
  expect(notes()!.closest("li")?.nextElementSibling).toBeNull();
});

test("draws no list for a Job that holds none, and asks again for the next walk's Job", async () => {
  await draw();
  await act(async () => push("s1"));
  await expect.poll(() => frame("s1")).not.toBeNull();
  expect(notes()).toBeNull();
  const before = asked.mock.calls.length;
  await act(async () => push("s2"));
  await expect.poll(() => frame("s2")).not.toBeNull();
  expect(asked.mock.calls.length).toBe(before + 1);
});

test("an edited note's words go to main as it is left, by the note's id, and nothing typed puts its words back", async () => {
  await holding();
  const field = note("Too tight")!.querySelector("textarea")!;
  await typed(field, "Too tight on the left");
  await act(async () => field.blur());
  expect(acts.editNote.mock.calls).toEqual([["n1", "Too tight on the left"]]);
  await typed(field, "   ");
  await act(async () => field.blur());
  expect(field.value).toBe("Too tight");
  expect(acts.editNote).toHaveBeenCalledTimes(1);
});

test("Remove takes the one note off by its id", async () => {
  await holding();
  await press(note("Wrong colour")!.querySelector("button"));
  expect(acts.removeNote.mock.calls).toEqual([["n2"]]);
});

test("Send notes sends them, and the list goes once main says the Job holds none", async () => {
  await holding();
  await press(send());
  expect(acts.sendNotes).toHaveBeenCalledTimes(1);
  await act(async () => tell([]));
  await expect.poll(() => notes()).toBeNull();
});

test("a send Fleet refused keeps every note listed and says why", async () => {
  acts.sendNotes.mockImplementation(async () => ({ ok: false, why: "not_connected" }));
  await holding();
  await press(send());
  await expect.poll(() => notes()?.querySelector('[role="alert"]')?.textContent).toBe("Not sent: Fleet is not connected. Nothing was sent.");
  expect(note("Too tight")).not.toBeNull();
  expect(note("Wrong colour")).not.toBeNull();
});

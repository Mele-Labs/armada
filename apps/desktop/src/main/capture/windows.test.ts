// Which question a Session's walk window shows — `windows.ts`'s `questionOf`, pure, so the rule is
// read without a window. `docs/concepts/session.md`, *Asking in the window*.

import { expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));
vi.mock("./window", () => ({ CaptureWindow: class {} }));

import type { SessionRecord, WaitingItem } from "@armada/protocol";
import { questionOf } from "./windows";

const AT = "2026-10-11T09:00:00Z";

const item = (id: string, url: string, source: WaitingItem["source"] = "window_ask"): WaitingItem => ({
  id,
  text: `question ${id}`,
  since: AT,
  source,
  act: { kind: "walk", target: "http://localhost:5173/" },
  options: [{ label: "Beside", description: "One line" }],
  place: { url, selector: "main > h1" },
});

const record = (waiting: WaitingItem[]): SessionRecord => ({
  id: "a",
  harness: "h",
  origin: "bridge",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  waiting_for: waiting,
});

it("shows the oldest open question on the window's own origin, and nothing else", () => {
  const shown = questionOf(
    record([
      item("walk:x", "http://localhost:5173/", "walk"),
      item("window_ask:q0", "http://localhost:6006/other"),
      item("window_ask:q1", "http://localhost:5173/findings"),
      item("window_ask:q2", "http://localhost:5173/later"),
    ]),
    ["http://localhost:5173"],
  );
  expect(shown).toEqual({
    id: "window_ask:q1",
    question: "question window_ask:q1",
    options: [{ label: "Beside", description: "One line" }],
    place: { url: "http://localhost:5173/findings", selector: "main > h1" },
  });
  expect(questionOf(record([]), ["http://localhost:5173"])).toBeNull();
  expect(questionOf(undefined, ["http://localhost:5173"])).toBeNull();
  expect(questionOf(record([item("window_ask:q1", "http://localhost:5173/")]), [])).toBeNull();
});

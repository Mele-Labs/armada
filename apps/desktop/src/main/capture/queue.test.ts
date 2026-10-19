// The walk window's queue as `queue.ts` reads it off the Jobs' waiting items: every walk standing, oldest shown first,
// one entry a walk. `docs/practices/capture-window.md`, *One window for every walk*.

import { expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));

import type { SessionRecord, WaitingItem } from "@armada/protocol";
import { sessionWalkKey, standingWalks } from "./queue";

const AT = "2026-10-19T09:00:00Z";
const at = (minute: string) => `2026-10-19T09:${minute}:00Z`;

const item = (url: string, since: string, source: WaitingItem["source"] = "walk"): WaitingItem => ({
  id: `${source}:${url}`,
  text: `Walk ${url}`,
  since,
  source,
  act: { kind: "walk", target: url },
});

const record = (id: string, ...waiting: WaitingItem[]): SessionRecord => ({
  id,
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

it("lists the walks of several Jobs oldest shown first, whichever Job listed them first", () => {
  const walks = standingWalks([
    record("a", item("https://a.example.com/late", at("05"))),
    record("b", item("https://b.example.com/", at("01"))),
    record("c", item("https://c.example.com/", at("03"))),
  ]);
  expect(walks.map((one) => one.sessionId)).toEqual(["b", "c", "a"]);
});

it("holds two walks of one Job as two entries, each in its own place among the others", () => {
  const walks = standingWalks([
    record("a", item("https://a.example.com/one", at("05")), item("https://a.example.com/two", at("02"))),
    record("b", item("https://b.example.com/", at("03"))),
  ]);
  expect(walks.map((one) => one.key)).toEqual([
    sessionWalkKey("a", "https://a.example.com/two"),
    sessionWalkKey("b", "https://b.example.com/"),
    sessionWalkKey("a", "https://a.example.com/one"),
  ]);
});

it("leaves out what is not a walk standing on a web address", () => {
  const walks = standingWalks([
    record(
      "a",
      item("https://a.example.com/", at("01"), "window_ask"),
      item("ftp://files.example.com/", at("02")),
      { id: "ask", text: "Which?", since: at("03"), source: "agent" },
      item("https://a.example.com/real", at("04")),
    ),
  ]);
  expect(walks.map((one) => one.url)).toEqual(["https://a.example.com/real"]);
});

// The line under a pull request's head: what counts as run on an older commit, and what Re-run runs again. His own asked questions are not among them.

import { expect, test, vi } from "vitest";

import type { PrWork, ReviewPr, ToolDef } from "./model";
import { rerunStale, stalePicks } from "./moved";
import { NO_WORK, type ReviewStore } from "./store";

const run = (state: string, head: string) => ({
  state,
  head,
});

const def = (id: string, over: Partial<ToolDef> = {}): ToolDef => ({
  id,
  kind: "tool",
  name: id,
  source: "armada",
  mark: "risks",
  checks: "",
  agent: true,
  ...over,
});

const work = {
  ...NO_WORK,
  runs: {
    risks: run("done", "old"),
    walk: run("done", "old"),
    fresh: run("done", "new"),
    going: run("running", "old"),
  },
  explains: {
    "how-it-works": run("done", "old"),
  },
  asks: [
    {
      id: "q1",
      words: "Why a mutex?",
      run: run("done", "old"),
    },
  ],
} as unknown as PrWork;

test("what is kept on another commit is stale: tools and explanations that are done, and no question of his", () => {
  expect(stalePicks(work, "new")).toEqual([
    "how-it-works",
    "risks",
    "walk",
  ]);
  expect(stalePicks(work, "old")).toEqual([
    "fresh",
  ]);
});

test("Re-run runs each stale tool and explanation again with nothing asked, and not his questions", () => {
  const calls = {
    explainAgain: vi.fn(),
    rerun: vi.fn(),
    findPlace: vi.fn(),
    askAgain: vi.fn(),
    spendUnasked: vi.fn((go: () => void, _again: boolean) => go()),
  };
  const defs = [
    def("risks"),
    def("walk", {
      mark: "walk",
    }),
    def("how-it-works", {
      kind: "explanation",
    }),
    def("fresh"),
  ];
  rerunStale(
    calls as unknown as ReviewStore,
    {
      key: "k",
    } as ReviewPr,
    defs,
    stalePicks(work, "new"),
  );
  expect(calls.rerun).toHaveBeenCalledTimes(1);
  expect(calls.rerun).toHaveBeenCalledWith("k", defs[0]);
  expect(calls.findPlace).toHaveBeenCalledWith("k");
  expect(calls.explainAgain).toHaveBeenCalledWith("k", defs[2]);
  expect(calls.askAgain).not.toHaveBeenCalled();
  // Pressed by him, so each goes through the unasked path without turning the asking off.
  expect(calls.spendUnasked).toHaveBeenCalledTimes(3);
  for (const [, again] of calls.spendUnasked.mock.calls) expect(again).toBe(false);
});

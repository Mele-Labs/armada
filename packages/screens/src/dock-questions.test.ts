// The cards for held calls, from what main gathered: each names its repository and offers the answers
// the call does, in Fleet's order.

import { describe, expect, it, vi } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { dockQuestionsOf } from "./dock-questions";
import type { Outstanding } from "./outstanding";

const manifest = (id: string, root: string) => ({
  id,
  repository: id,
  path: `${root}/armada.yml`,
  records_root: `/r/${id}`,
  version: 1,
  checks: [],
});
const REPOSITORIES: RepositorySummary[] = [
  {
    root: "/u/armada",
    records_root: "/r/armada",
    manifest: manifest("armada", "/u/armada"),
  },
  {
    root: "/u/shop",
    records_root: "/r/shop",
    manifest: manifest("shop-01", "/u/shop"),
  },
];

// #1389: a card drawn from a held call names its repository and nothing else.
const HELD: Outstanding = {
  kind: "call",
  call: {
    call: "call-1",
    manifest_id: "shop-01",
    asked_at: "2026-09-13T10:02:00Z",
    tool: "Bash",
    detail: "gh issue list --milestone Release",
    truncated: false,
    length: 30,
    rule: "Bash(gh issue list:*)",
    offers: [
      "allow_once",
      "allow_and_remember",
      "refuse",
    ],
    holding_for_seconds: 300,
  },
};

const LATER: Outstanding = {
  kind: "call",
  call: {
    ...HELD.call,
    call: "call-2",
    asked_at: "2026-09-13T10:05:00Z",
  },
};

describe("a held call as a card", () => {
  it("names its repository and the rule it would need", () => {
    const [card] = dockQuestionsOf(
      [
        HELD,
      ],
      REPOSITORIES,
      Date.parse("2026-09-13T10:03:00Z"),
    );

    expect(card?.repository).toBe("shop-01");
    expect(card?.label).toBe("A job needs your permission");
    expect(card?.detail).toBe("Your settings would have to allow Bash(gh issue list:*).");
    expect(card?.answers.map((answer) => answer.id)).toEqual([
      "allow_once",
      "allow_and_remember",
      "refuse",
    ]);
  });

  it("is drawn oldest first", () => {
    const cards = dockQuestionsOf(
      [
        LATER,
        HELD,
      ],
      REPOSITORIES,
      Date.now(),
    );

    expect(cards.map((card) => card.id)).toEqual([
      "call:call-1",
      "call:call-2",
    ]);
  });

  it("sends its answer by the wire's own word", () => {
    const onAnswer = vi.fn();
    const [card] = dockQuestionsOf(
      [
        HELD,
      ],
      REPOSITORIES,
      Date.now(),
      {
        onAnswer,
      },
    );

    card?.onAnswer?.("allow_and_remember");

    expect(onAnswer).toHaveBeenCalledWith(HELD, "allow_and_remember");
  });

  it("says where to answer until answering is handed in", () => {
    const [card] = dockQuestionsOf(
      [
        HELD,
      ],
      REPOSITORIES,
      Date.now(),
    );

    expect(card?.onAnswer).toBeUndefined();
    expect(card?.note).toBe("Open Armada on this machine to answer.");
  });

  it("draws nothing where nothing is waiting", () => {
    expect(dockQuestionsOf([], REPOSITORIES, Date.now())).toEqual([]);
  });
});

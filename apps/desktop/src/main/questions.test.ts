// Helm calls waiting on a person: its own event puts one up, its own event takes it down, and a
// resync reads them from their own route. #1389.

import { describe, expect, it } from "vitest";

import { NOTHING_YET, type BridgeState } from "../shared/bridge";
import { Questions } from "./questions";

function holding() {
  let state: BridgeState = { ...NOTHING_YET };
  const questions = new Questions({
    current: () => state,
    publish: (change) => (state = { ...state, ...change }),
  });
  return { questions, state: () => state };
}

describe("a helm call waiting on a person", () => {
  const WAITING = {
    call: "helm-1",
    manifest_id: "armada",
    asked_at: "2026-09-13T10:02:00Z",
    tool: "Bash",
    detail: "gh issue list",
    truncated: false,
    rule: "Bash(gh issue list:*)",
    offers: ["allow_once", "allow_and_remember", "refuse"] as const,
    holding_for_seconds: 300,
  };

  it("goes up on its ask and comes down on its answer", () => {
    const { questions, state } = holding();
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    expect(state().questions.map((question) => question.kind)).toEqual(["helm"]);

    questions.helmAnswered("helm-1");
    expect(state().questions).toEqual([]);
  });

  it("is drawn once when its ask arrives twice", () => {
    const { questions, state } = holding();
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    expect(state().questions).toHaveLength(1);
  });

  it("keeps what is held when the read fails", async () => {
    const { questions, state } = holding();
    questions.helmAsking({ ...WAITING, offers: [...WAITING.offers] });
    await questions.readHelmCalls(1);
    expect(state().questions.map((question) => question.kind)).toEqual(["helm"]);
  });
});

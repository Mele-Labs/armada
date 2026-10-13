// Helm's approval ask, folded from the tool row into the card: which rows become one, what the card reads from the
// Board, and what its press does. The look is the walk `helm-approve-card`.

import { describe, expect, it, vi } from "vitest";

import type { SessionThreadRow } from "@armada/components";
import type { JobSummary, WorkflowSummary } from "@armada/protocol";

import { approvalAskOf, withApprovals } from "./helm-approval";
import type { Approving } from "./helm-approval";

const job = (status: string) => ({ id: "j1", handle: "81-retire-the-poke", status, workflow_id: "feature", owner_manifest_id: "armada" }) as unknown as JobSummary;
const workflow = { id: "feature", manifest_id: "armada", name: "Feature", steps: [{}, {}, {}] } as unknown as WorkflowSummary;

const tool = (text: string): SessionThreadRow => ({ id: "t1", at: "13:59:10", kind: "tool", text });

function approving(over: Partial<Approving> = {}): Approving {
  return { jobs: [job("awaiting_approval")], workflows: [workflow], pressed: {}, onApprove: async () => ({ ok: true }), onPress: () => undefined, ...over };
}

const cardOf = (rows: readonly SessionThreadRow[]) => {
  const row = rows[0];
  if (row?.kind !== "approval") throw new Error(`expected an approval row, got ${row?.kind}`);
  return row.card;
};

describe("the tool rows that are an approval ask", () => {
  it("is the Job id after the tool's name, with or without the server's prefix", () => {
    expect(approvalAskOf("ask_person_to_approve j1")).toBe("j1");
    expect(approvalAskOf("mcp__armada-fleet__ask_person_to_approve j1")).toBe("j1");
  });

  it("is nothing for any other tool, or for a call that names no Job", () => {
    expect(approvalAskOf("mcp__armada-fleet__list_jobs")).toBeUndefined();
    expect(approvalAskOf("mcp__armada-fleet__get_job j1")).toBeUndefined();
    expect(approvalAskOf("ask_person_to_approve")).toBeUndefined();
    expect(approvalAskOf("ask_person_to_approve j1 j2")).toBeUndefined();
    expect(approvalAskOf("not_ask_person_to_approve j1")).toBeUndefined();
  });

  it("becomes an approval row in the place of the tool row, with the same id and time", () => {
    for (const text of ["ask_person_to_approve j1", "mcp__armada-fleet__ask_person_to_approve j1"]) {
      const [row] = withApprovals([tool(text)], approving());
      expect(row).toMatchObject({ id: "t1", at: "13:59:10", kind: "approval" });
    }
  });

  it("leaves every other row as it was, the same object", () => {
    const other = tool("mcp__armada-fleet__list_jobs");
    const message: SessionThreadRow = { id: "m1", at: "13:59:00", kind: "message", from: "you", text: "what is waiting on me?" };
    const rows = [message, other];
    const drawn = withApprovals(rows, approving());
    expect(drawn[0]).toBe(message);
    expect(drawn[1]).toBe(other);
  });
});

describe("the card an ask draws", () => {
  it("reads the Job's handle, workflow and steps from the Board", () => {
    expect(cardOf(withApprovals([tool("ask_person_to_approve j1")], approving()))).toMatchObject({ jobHandle: "81-retire-the-poke", workflow: "Feature", stepCount: 3, state: "ready" });
  });

  it("is ready at the gate, and elsewhere once the Job has moved by any other road", () => {
    expect(cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ jobs: [job("awaiting_approval")] }))).state).toBe("ready");
    expect(cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ jobs: [job("running")] }))).state).toBe("elsewhere");
  });

  it("is unknown for a Job the Board has never heard of, and names the id", () => {
    const card = cardOf(withApprovals([tool("ask_person_to_approve nope")], approving()));
    expect(card).toMatchObject({ jobHandle: "nope", state: "unknown" });
    expect(card.onApprove).toBeUndefined();
  });

  it("is a press made here over the Board, so an approved card does not snap to elsewhere", () => {
    const moved = approving({ jobs: [job("running")], pressed: { t1: "approved" } });
    expect(cardOf(withApprovals([tool("ask_person_to_approve j1")], moved)).state).toBe("approved");
    expect(cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ pressed: { t1: "dismissed" } }))).state).toBe("dismissed");
  });

  it("has no press once it is answered", () => {
    const card = cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ pressed: { t1: "approved" } })));
    expect(card.onApprove).toBeUndefined();
    expect(card.onDismiss).toBeUndefined();
  });
});

describe("the presses on a card", () => {
  it("Approve presses the Job's gate and holds approved once Fleet answers ok", async () => {
    const onApprove = vi.fn(async () => ({ ok: true }));
    const onPress = vi.fn();
    cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ onApprove, onPress }))).onApprove?.();
    expect(onApprove).toHaveBeenCalledWith("j1");
    expect(onPress).toHaveBeenNthCalledWith(1, "t1", "pending");
    await vi.waitFor(() => expect(onPress).toHaveBeenNthCalledWith(2, "t1", "approved"));
  });

  it("drops the press when Fleet refuses, so the card reads the Board again", async () => {
    const onPress = vi.fn();
    cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ onApprove: async () => ({ ok: false }), onPress }))).onApprove?.();
    await vi.waitFor(() => expect(onPress).toHaveBeenNthCalledWith(2, "t1", undefined));
  });

  it("Not now holds dismissed and does not touch the Job", () => {
    const onApprove = vi.fn(async () => ({ ok: true }));
    const onPress = vi.fn();
    cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ onApprove, onPress }))).onDismiss?.();
    expect(onPress).toHaveBeenCalledWith("t1", "dismissed");
    expect(onApprove).not.toHaveBeenCalled();
  });

  it("a card pressed and not yet answered is pending, and keeps Approve bound", () => {
    const card = cardOf(withApprovals([tool("ask_person_to_approve j1")], approving({ pressed: { t1: "pending" } })));
    expect(card.state).toBe("pending");
    expect(card.onApprove).toBeDefined();
  });
});

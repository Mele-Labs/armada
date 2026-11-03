import { describe, expect, test } from "vitest";

import { chipCardOf } from "./chip-card";
import type { Dot } from "../horizon";

const dot = (card: Partial<Dot["card"]>): Dot => ({
  key: "pull:7",
  state: "running",
  queued: true,
  icon: null,
  tip: "#7",
  card: {
    heading: "#7",
    title: "Title",
    branch: "feat/seven",
    says: "In the merge queue",
    check: undefined,
    place: 1,
    owner: undefined,
    ...card,
  },
  act: undefined,
});

describe("the merge line chip's card", () => {
  test("says how long the pull request has been open, changed and queued, as ages", () => {
    const facts = chipCardOf(
      dot({
        opened: "2026-11-03T09:00:00Z",
        updated: "2026-11-03T14:50:00Z",
        queuedAt: "2026-11-03T14:00:00Z",
      }),
    ).facts;
    expect(facts.filter((fact) => ["Opened", "Updated", "In queue"].includes(fact.label))).toEqual([
      { label: "Opened", value: { at: "2026-11-03T09:00:00Z", age: true } },
      { label: "Updated", value: { at: "2026-11-03T14:50:00Z", age: true } },
      { label: "In queue", value: { at: "2026-11-03T14:00:00Z", age: true } },
    ]);
  });

  test("draws a time row only where the forge gave one", () => {
    const labels = chipCardOf(dot({})).facts.map((fact) => fact.label);
    expect(labels).not.toContain("Opened");
    expect(labels).not.toContain("Updated");
    expect(labels).not.toContain("In queue");
  });
});

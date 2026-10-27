import { describe, expect, test } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { jobRow } from "./palette-job-row";

const job = (over: Partial<Session>): Session => ({ id: "j1", turn: { state: "idle" }, standing: "idle", rows: [], attachments: [], ...over }) as Session;
const repoOf = (manifestId: string) => ({ armada: "armada", pocket: "Pocket" })[manifestId as "armada" | "pocket"];

describe("a Job as a palette row", () => {
  test("says which repository it works in, from the Manifest it carries", () => {
    expect(jobRow(job({ manifestId: "pocket" }), repoOf).repo).toBe("Pocket");
    expect(jobRow(job({ manifestId: "unknown" }), repoOf).repo).toBeUndefined();
    expect(jobRow(job({}), repoOf).repo).toBeUndefined();
  });

  test("is found by its title, its id and its address", () => {
    const row = jobRow(job({ id: "9ca3", title: "Fix the merge line", address: "s-9ca3" }), repoOf);
    expect(row.label).toBe("Fix the merge line");
    expect(row.aliases).toEqual(["9ca3", "s-9ca3"]);
  });

  test("states how it stands as a glyph with the words its tooltip says, and marks a working Job to pulse", () => {
    const working = jobRow(job({ turn: { state: "working" }, standing: "working" }), repoOf);
    expect(working.state).toMatchObject({ said: "Working", pulsing: true });
    const paused = jobRow(job({ paused: true }), repoOf);
    expect(paused.state).toMatchObject({ said: "Paused" });
    expect(paused.state?.pulsing).toBeUndefined();
    expect(paused.paused).toBe(true);
  });

  test("holds the number of the pull request it has, which finds it and is not drawn", () => {
    const pull = { kind: "pull_request", number: 1899, title: "t", branch: "b", address: "a", checks: { state: "passed" }, state: "open", auto: false } as Session["attachments"][number];
    expect(jobRow(job({ attachments: [pull] }), repoOf).pull).toBe(1899);
    expect(jobRow(job({}), repoOf).pull).toBeUndefined();
  });
});

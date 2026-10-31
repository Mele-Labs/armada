// Which Sessions and held slots the pick lists.

import { describe, expect, it } from "vitest";

import { heldOfPicked, heldOfWorkspace, sessionsOfPicked, sessionsOfWorkspace } from "./board";

import type { SessionRecord, WorktreeSlot, WorktreesHeld } from "@armada/protocol";

describe("which Sessions and held slots the pick lists", () => {
  const session = (id: string, manifest_id?: string) => ({ id, ...(manifest_id === undefined ? {} : { manifest_id }) }) as unknown as SessionRecord;
  const ours = session("ours", "armada");
  const theirs = session("theirs", "ynap");
  const loose = session("loose");
  const armada = { manifest: { id: "armada" } };

  it("lists only the picked repository's Sessions, and every one on All", () => {
    expect(sessionsOfPicked([ours, theirs, loose], armada)).toEqual([ours]);
    expect(sessionsOfPicked([ours, theirs, loose], null)).toEqual([ours, theirs, loose]);
  });

  it("lists no Session for a picked repository that has no Manifest yet", () => {
    expect(sessionsOfPicked([ours, loose], {})).toEqual([]);
  });

  it("lists Setup's Session, which has no Manifest, on the repository whose root it stands in", () => {
    const setup = { ...session("setup"), cwd: "/r/new" } as unknown as SessionRecord;
    expect(sessionsOfPicked([ours, loose, setup], { root: "/r/new" })).toEqual([setup]);
    expect(sessionsOfPicked([setup], { root: "/r/other" })).toEqual([]);
    expect(sessionsOfPicked([setup], { ...armada, root: "/r/new" })).toEqual([setup]);
    expect(sessionsOfPicked([setup], armada)).toEqual([]);
  });

  const slot = (manifest_id: string, n: number) => ({ manifest_id, slot: n }) as unknown as WorktreeSlot;
  const held: WorktreesHeld = { slots: [slot("armada", 1), slot("ynap", 2)] };

  it("keeps the picked repository's slots", () => {
    expect(heldOfPicked(held, armada)).toEqual({ slots: [slot("armada", 1)] });
  });

  it("keeps everything on All", () => {
    expect(heldOfPicked(held, null)).toBe(held);
  });

  const ynap = { manifest: { id: "ynap" } };

  it("lists the Sessions of every repository a Workspace holds, and none of another", () => {
    const other = session("other", "elsewhere");
    expect(sessionsOfWorkspace([ours, theirs, other, loose], [armada, ynap])).toEqual([ours, theirs]);
    expect(sessionsOfWorkspace([ours, theirs], [])).toEqual([]);
  });

  it("keeps the slots of every repository a Workspace holds", () => {
    expect(heldOfWorkspace({ slots: [slot("armada", 1), slot("ynap", 2), slot("elsewhere", 3)] }, [armada, ynap])).toEqual({ slots: [slot("armada", 1), slot("ynap", 2)] });
    expect(heldOfWorkspace({}, [armada])).toEqual({});
  });
});

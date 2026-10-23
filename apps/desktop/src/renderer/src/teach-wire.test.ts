// What a press on something becomes in Fleet's `read_teaching` and `save_teaching`, and what the panel draws back of a proposal when
// the Fleet that read it says no more than the wire does.

import { describe, expect, it } from "vitest";
import type { TeachingProposal } from "@armada/protocol";

import type { PanelItem } from "./shared-panel/store";
import { BRIDGE } from "./teach";
import { previewOf, readBody, readingOf, saveBody, sourceOf } from "./teach-wire";

type Pressed = Extract<PanelItem, { kind: "teach" }>;

const press = (over: Partial<Pressed> = {}): Pressed => ({ kind: "teach", sessionId: "01JOB", jobTitle: "Pin the clock", manifestId: "armada", root: "/code/armada", repository: "armada", source: "words", text: "STORE-1234", ...over });

describe("what Teach was pressed on", () => {
  it("is the thing's text, and the ids that name it where there are any", () => {
    expect(sourceOf(press())).toEqual({ kind: "words", text: "STORE-1234" });
    expect(sourceOf(press({ source: "notice", text: "slot-14 is 25 commits behind main.", ids: ["behind_base"] }))).toEqual({ kind: "notice", text: "slot-14 is 25 commits behind main.", ids: ["behind_base"] });
    expect(sourceOf(press({ ids: [] }))).not.toHaveProperty("ids");
  });

  it("is named by what it is where it has no text of its own, as the title bar has none", () => {
    expect(sourceOf(press({ source: "title_bar", text: "" }))).toEqual({ kind: "title_bar", text: "Title bar" });
  });

  it("is cut at the 200 characters Fleet takes", () => {
    expect(sourceOf(press({ text: "x".repeat(500) })).text).toHaveLength(200);
  });
});

describe("read_teaching's body", () => {
  it("names the repository's root for a teaching that reaches one repository, and the Session it was taught in", () => {
    expect(readBody(press(), "A Jira key", "repository")).toEqual({
      words: "STORE-1234",
      said: "A Jira key",
      reach: "repository",
      source: { kind: "words", text: "STORE-1234" },
      repository: "/code/armada",
      session_id: "01JOB",
    });
  });

  it("names no repository for every repository, which is keyed to none", () => {
    const body = readBody(press(), "A Jira key", "all");
    expect(body.reach).toBe("everywhere");
    expect(body).not.toHaveProperty("repository");
    expect(body.session_id).toBe("01JOB");
  });

  it("carries the selected words only where words were selected, and names no Session for Bridge's own screen", () => {
    const body = readBody(press({ sessionId: BRIDGE, source: "row", text: "Round the payout totals" }), "make these louder", "all");
    expect(body).not.toHaveProperty("words");
    expect(body).not.toHaveProperty("session_id");
    expect(body.source).toEqual({ kind: "row", text: "Round the payout totals" });
  });
});

const TWEAK = { token: "--status-awaiting-review", was: "#old", now: "#new", used_in: ["--run-waiting"], note: "Needs you amber" };

describe("save_teaching's body", () => {
  const proposal: TeachingProposal = { note: { section: "repo", kind: "convention", text: "Tickets are Jira keys." }, tweak: [TWEAK, { ...TWEAK, token: "--other" }] };

  it("is the proposal as it was shown, with the first colour only, and where it was taught", () => {
    const body = saveBody(press({ source: "command" }), proposal, "repository");
    expect(body.note).toEqual(proposal.note);
    expect(body.tweak).toEqual([TWEAK]);
    expect(body).toMatchObject({ reach: "repository", cite: "Teach, command", repository: "/code/armada", session_id: "01JOB", source: { kind: "command", text: "STORE-1234" } });
  });

  it("names no repository for every repository", () => {
    expect(saveBody(press(), proposal, "all")).not.toHaveProperty("repository");
  });
});

describe("what the panel draws of a proposal", () => {
  it("is the note, the words with their example in the address, and the colour as Fleet named it", () => {
    const reading = readingOf({ note: { section: "repo", kind: "convention", text: "Tickets are Jira keys." }, words: { name: "Jira key", pattern: "[A-Z]+-\\d+", example: "STORE-1234", address: { template: "https://acme.atlassian.net/browse/{match}", found: "settings" } }, tweak: [TWEAK] }, "armada");
    expect(reading.agents).toBe("Tickets are Jira keys.");
    expect(reading.bridge).toEqual({ name: "Jira key", pattern: "[A-Z]+-\\d+", example: "STORE-1234", address: "https://acme.atlassian.net/browse/STORE-1234" });
    expect(reading.tweak).toEqual({ colour: "Needs you amber", token: "--status-awaiting-review", was: "#old", now: "#new", places: ["--run-waiting"] });
  });

  it("says no address where Fleet found none, since it finds one when the teaching is kept", () => {
    expect(readingOf({ words: { name: "Jira key", pattern: "x", example: "STORE-1234" } }, "armada").bridge).not.toHaveProperty("address");
  });

  it("names a notice by its word as the menu does, what Fleet does with it, and where", () => {
    const rule = (where: string) => readingOf({ fleet: { notice: "behind_base", does: "batch", where } }, "armada").fleet;
    expect(rule("repo armada")).toEqual({ notice: "Slot behind notice", does: "Batch", scope: "armada" });
    expect(rule("everywhere")?.scope).toBe("Every repository");
    expect(rule("job 01JOB")?.scope).toBe("This Job");
  });
});

describe("what a proposal draws behind the panel before it is kept", () => {
  it("is the link where an address came with the words, and the colour", () => {
    const words = { name: "Jira key", pattern: "[A-Z]+-\\d+", example: "STORE-1234", address: { template: "https://acme.atlassian.net/browse/{match}", found: "settings" } };
    expect(previewOf({ words, tweak: [TWEAK] }, "all", "armada", undefined)).toEqual({
      reach: "all",
      manifestId: "armada",
      link: { pattern: "[A-Z]+-\\d+", address: "https://acme.atlassian.net/browse/{match}" },
      tweak: { token: "--status-awaiting-review", now: "#new" },
    });
  });

  it("is nothing where the proposal changes neither", () => {
    expect(previewOf({ note: { section: "repo", kind: "convention", text: "x" } }, "repository", "armada", undefined)).toBeUndefined();
  });
});

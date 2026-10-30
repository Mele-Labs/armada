// What a press on something becomes in Fleet's `read_teaching` and `save_teaching`, and what the panel draws back of a proposal when
// the wire carries.

import { describe, expect, it } from "vitest";
import type { AppliedLifecycle, LifecycleRule, TeachingProposal } from "@armada/protocol";

import type { PanelItem } from "./shared-panel/store";
import { BRIDGE } from "./teach";
import { formLifecycles, previewOf, readBody, readingOf, saveBody, sourceOf } from "./teach-wire";

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

  it("is a ticket by its key, with the key and its title as the text", () => {
    expect(sourceOf(press({ source: "ticket", text: "ARM-41 Add a retry", ids: ["ARM-41"] }))).toEqual({ kind: "ticket", text: "ARM-41 Add a retry", ids: ["ARM-41"] });
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
    expect(reading.bridge).toEqual({ name: "Jira key", pattern: "[A-Z]+-\\d+", example: "STORE-1234", address: "https://acme.atlassian.net/browse/STORE-1234", found: "Found in this repository's settings" });
    expect(reading.tweak).toEqual({ colour: "Needs you amber", token: "--status-awaiting-review", was: "#old", now: "#new", places: ["--run-waiting"] });
  });

  it("says where the address was found, as the wire spells it", () => {
    const found = (word: string) => readingOf({ words: { name: "Jira key", pattern: "x", example: "STORE-1234", address: { template: "https://acme.atlassian.net/browse/{match}", found: word } } }, "armada").bridge?.found;
    expect(found("pull_request #2712")).toBe("Found in pull request #2712");
    expect(found("person")).toBe("You said");
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

  it("draws the pattern a rule matches and each condition after what it does", () => {
    const fleet = { notice: "behind_base", does: "batch" as const, where: "repo armada", matches: "\\d+ commits behind main", when: [{ if: "blocks_merge" as const, does: "tell" as const }] };
    expect(readingOf({ fleet }, "armada").fleet).toEqual({
      notice: "Slot behind notice",
      matches: "\\d+ commits behind main",
      does: "Batch, and tell only when it blocks a merge",
      scope: "armada",
    });
  });

  it("says what Fleet cannot test in one line of the person's own words, and nothing where there is none", () => {
    const fleet = { notice: "behind_base", does: "batch" as const, where: "everywhere" };
    expect(readingOf({ fleet: { ...fleet, untestable: ["the build is green", "it is Friday"] } }, "armada").fleet?.untestable).toBe("Fleet cannot test: the build is green, it is Friday");
    expect(readingOf({ fleet: { ...fleet, untestable: [] } }, "armada").fleet).not.toHaveProperty("untestable");
    expect(readingOf({ fleet }, "armada").fleet).not.toHaveProperty("matches");
  });
});

describe("what a proposal draws behind the panel before it is kept", () => {
  it("is the link where an address came with the words, and the colour", () => {
    const words = { name: "Jira key", pattern: "[A-Z]+-\\d+", example: "STORE-1234", address: { template: "https://acme.atlassian.net/browse/{match}", found: "settings" } };
    expect(previewOf({ words, tweak: [TWEAK] }, "all", "armada")).toEqual({
      reach: "all",
      manifestId: "armada",
      link: { pattern: "[A-Z]+-\\d+", address: "https://acme.atlassian.net/browse/{match}", from: "Found in this repository's settings" },
      tweak: { token: "--status-awaiting-review", now: "#new" },
    });
  });

  it("is nothing where the proposal changes neither", () => {
    expect(previewOf({ note: { section: "repo", kind: "convention", text: "x" } }, "repository", "armada")).toBeUndefined();
  });
});

const RULE: LifecycleRule = {
  id: "move_its_tickets_to_done",
  name: "Move its tickets to Done",
  scope: { kind: "within", id: "armada" },
  when: { kind: "enters", lifecycle: "pull_request", state: "approved" },
  conditions: [],
  does: [{ what: { act: "tracker_move", to: "closed", target: { among: { kind: "linked_from", lifecycle: "ticket" } } }, kind: "tracker_write", autonomy: "asks" }],
  autonomy: "asks",
  on_failure: { retries: 2, then: "ask_owner" },
};

describe("a rule in a proposal", () => {
  it("is drawn as the rule alone, with the lifecycles its pickers offer, and what Fleet cannot test in one line", () => {
    const lifecycles = [{ id: "ticket", name: "Ticket", kind: "tracker" as const, states: [{ id: "closed", label: "Closed" }] }];
    const { rule } = readingOf({ rule: { ...RULE, untestable: ["the build is green"] } }, "armada", lifecycles);
    expect(rule?.rule).toEqual(RULE);
    expect(rule?.lifecycles).toBe(lifecycles);
    expect(rule?.untestable).toBe("Fleet cannot test: the build is green");
  });

  it("says nothing of what Fleet cannot test where there is none, and offers the steps and workflow the rule names and no others", () => {
    const named: LifecycleRule = {
      ...RULE,
      when: { kind: "step_ends", step: "build" },
      conditions: [
        { kind: "at_step", step: "review" },
        { kind: "workflow", workflow: "ship" },
      ],
    };
    const { rule } = readingOf({ rule: named }, "armada");
    expect(rule).not.toHaveProperty("untestable");
    expect(rule?.steps).toEqual([
      { id: "build", label: "build" },
      { id: "review", label: "review" },
    ]);
    expect(rule?.workflows).toEqual(["ship"]);
  });

  it("is kept as the person changed it, with the same word for what could not be tested", () => {
    const body = saveBody(press({ source: "pull_request", text: "#2712 Pin the clock" }), { rule: { ...RULE, autonomy: "acts", untestable: ["it is Friday"] } }, "repository");
    expect(body.rule).toMatchObject({ id: "move_its_tickets_to_done", autonomy: "acts", untestable: ["it is Friday"] });
    expect(body.source).toEqual({ kind: "pull_request", text: "#2712 Pin the clock" });
  });

  it("draws behind the panel nothing of its own", () => {
    expect(previewOf({ rule: RULE }, "repository", "armada")).toBeUndefined();
  });
});

describe("the lifecycles a rule's pickers offer", () => {
  const applied = (id: string, name: string, reach: string, source: "tracker" | "forge", extra: Partial<AppliedLifecycle["kept"]> = {}): AppliedLifecycle => ({
    lifecycle: {
      id,
      name,
      scope: reach === "everywhere" ? { kind: "everywhere" } : { kind: "within", id: "armada" },
      states: [{ id: "a", label: `${name} A`, read: source === "tracker" ? { source, words: ["A"] } : { source, fact: "draft" } }],
      transitions: [],
      ends: [],
    },
    kept: { rank: { reach, source: "kit" }, ...extra },
  });

  it("are one for each id, the narrowest that was not replaced, each of the kind its first state is read by", () => {
    const got = formLifecycles([
      applied("ticket", "Ticket", "everywhere", "tracker"),
      applied("ticket", "Ticket in QA", "repository", "tracker"),
      applied("pull_request", "Pull request", "everywhere", "forge"),
      applied("pull_request", "Replaced", "repository", "forge", { overridden_by: { reach: "repository", source: "repository" } }),
    ]);
    expect(got).toEqual([
      { id: "ticket", name: "Ticket in QA", kind: "tracker", states: [{ id: "a", label: "Ticket in QA A" }] },
      { id: "pull_request", name: "Pull request", kind: "forge", states: [{ id: "a", label: "Pull request A" }] },
    ]);
  });
});

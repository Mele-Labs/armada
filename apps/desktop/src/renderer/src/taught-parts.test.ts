// What a taught mod holds, as its row opens to it and a Logbook entry words it, from the file Fleet checked.

import { describe, expect, it } from "vitest";
import type { ModChecked } from "@armada/protocol";

import { fleetAlso, partsOf } from "./taught-parts";

const checked = (over: Partial<ModChecked>): ModChecked => ({ name: "a-mod", valid: true, problems: [], ...over });

describe("a Words mod", () => {
  it("holds its pattern and an address for each repository, with where each was found", () => {
    const parts = partsOf(
      checked({
        words: {
          name: "Jira key",
          pattern: "[A-Z]+-\\d+",
          example: "STORE-1234",
          address: [
            { repository: "armada", template: "https://acme.atlassian.net/browse/{match}", found: "settings" },
            { repository: "payments", template: "https://pay.atlassian.net/browse/{match}", found: "pull_request #2712" },
            { repository: "*", template: "https://other.atlassian.net/browse/{match}", found: "person" },
          ],
        },
      }),
    );
    expect(parts).toEqual([
      {
        part: "Words",
        facts: [
          { label: "Pattern", value: "[A-Z]+-\\d+", code: true },
          { label: "armada", value: "https://acme.atlassian.net/browse/{match}", code: true, from: "Found in this repository's settings" },
          { label: "payments", value: "https://pay.atlassian.net/browse/{match}", code: true, from: "Found in pull request #2712" },
          { label: "Other repositories", value: "https://other.atlassian.net/browse/{match}", code: true, from: "You said" },
        ],
      },
    ]);
  });
});

describe("a Fleet mod", () => {
  const fleet = { rule: [{ notice: "behind_base", does: "batch" as const, where: "repo armada" }] };

  it("holds each rule: the notice by its word, what is done with it and where", () => {
    expect(partsOf(checked({ fleet }))).toEqual([
      { part: "Fleet", facts: [{ label: "Notice", value: "Slot behind notice" }, { label: "Does", value: "Batch" }, { label: "Where", value: "armada" }] },
    ]);
  });

  it("is worded in a line for the entry that made it", () => {
    expect(fleetAlso(fleet)).toBe("slot behind notice, batched for this repository");
    expect(fleetAlso({ rule: [{ notice: "behind_base", does: "quiet", where: "everywhere" }] })).toBe("slot behind notice, quiet for every repository");
    expect(fleetAlso({ rule: [{ notice: "behind_base", does: "call", where: "job 01JOB" }] })).toBe("slot behind notice, called to you for this Job");
    expect(fleetAlso({})).toBeUndefined();
  });
});

describe("a tweak and a layout", () => {
  it("hold the tokens the stylesheet sets", () => {
    expect(partsOf(checked({ css: ":root { --status-awaiting-review: #FFC233; }" }))).toEqual([
      { part: "Theme tweak", facts: [{ label: "Token", value: "--status-awaiting-review", code: true }, { label: "Now", value: "#FFC233", code: true }] },
    ]);
  });

  it("hold what the layout hides, and where, as the registry names them", () => {
    expect(partsOf(checked({ layout: JSON.stringify({ version: 1, rail: { hidden: ["guides"] } }) }))).toEqual([
      { part: "Layout", facts: [{ label: "Hides", value: "Guides" }, { label: "In", value: "Places" }] },
    ]);
  });
});

it("holds nothing of a mod that did not pass", () => {
  expect(partsOf(checked({ valid: false, problems: ["line 1"] }))).toEqual([]);
});

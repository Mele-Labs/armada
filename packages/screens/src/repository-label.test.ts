// What a repository is called in the picker — #886 — and on a Board row on All repositories.

import { describe, expect, it } from "vitest";

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { manifestLabel, repositoryLabel } from "@armada/shell/src/repository-label";
import { BOARD_COLUMNS, columnsFor, REPOSITORY_COLUMN, repositoryOf, TASKS_COLUMN } from "./board";

/** A ULID, because `ManifestId` is one — the id is never the label. */
const MINTED = "01M1CNPKTV0018H2M1CXDNBK06";

const manifest = (named: string, root: string, id = MINTED) => ({ id, repository: named, path: `${root}/armada.yml`, records_root: `/records/${named}`, version: 1, checks: [] });
const setUp = (named: string, root: string, id = MINTED): RepositorySummary => ({ root, records_root: `/records/${named}`, manifest: manifest(named, root, id) });
const loose = (root: string): RepositorySummary => ({ root, records_root: `/records${root}` });

describe("the picker's label", () => {
  it("reads a set-up repository as the name Fleet read it under, never its minted id", () => {
    const every = [setUp("storefront", "/Users/user/code/storefront"), loose("/Users/user/old/api")];
    expect(repositoryLabel(every[0]!, every)).toBe("storefront");
    expect(repositoryLabel(every[0]!, every)).not.toContain(MINTED);
  });

  it("takes the Manifest's own name over the folder, which is the workspace inside a monorepo", () => {
    const every = [setUp("billing", "/Users/user/code/platform")];
    expect(repositoryLabel(every[0]!, every)).toBe("billing");
  });

  it("reads one not set up as its folder, even where a set-up name shares it", () => {
    const every = [setUp("api", "/Users/user/services/api"), loose("/Users/user/code/api/")];
    expect(repositoryLabel(every[1]!, every)).toBe("code/api");
  });

  it("widens on a collision whichever side is set up, since the labels share one namespace", () => {
    const every = [setUp("api", "/Users/user/services/api"), setUp("api", "/Users/user/old/api", "01M1OTHERMANIFEST0000000000")];
    expect(repositoryLabel(every[0]!, every)).toBe("services/api");
    expect(repositoryLabel(every[1]!, every)).toBe("old/api");
  });

  it("adds the parent only where two share a folder, and the root past that", () => {
    const every = [loose("/Users/user/code/api"), loose("/Users/user/old/api"), loose("/Volumes/code/api")];
    expect(repositoryLabel(every[1]!, every)).toBe("old/api");
    expect(repositoryLabel(every[0]!, every)).toBe("/Users/user/code/api");
    expect(repositoryLabel(every[2]!, every)).toBe("/Volumes/code/api");
  });

  it("names a Job's repository by the label of the Manifest it carries, and a bare id where that Manifest is gone", () => {
    const every = [setUp("armada", "/Users/user/armada")];
    expect(manifestLabel(MINTED, every)).toBe("armada");
    // The record keeps its last known name and the wire does not carry one for a
    // Manifest nobody serves, so the id is the only honest thing left to draw.
    expect(manifestLabel("01M1GONE00000000000000000000", every)).toBe("01M1GONE00000000000000000000");
  });
});

describe("a Board row's repository", () => {
  const STOREFRONT = "01M1STOREFRONT00000000000000";
  const job = { owner_manifest_id: STOREFRONT } as JobSummary;
  const two = [setUp("armada", "/Users/user/armada"), setUp("storefront", "/Users/user/storefront", STOREFRONT)];

  it("is named, with its column, on All where Fleet serves more than one", () => {
    expect(repositoryOf(job, two, true)).toBe("storefront");
    expect(columnsFor([], two, true)).toEqual([...BOARD_COLUMNS, REPOSITORY_COLUMN]);
    for (const served of [null, [], two.slice(1)]) {
      expect(repositoryOf(job, served, true)).toBeUndefined();
      expect(columnsFor([], served, true)).toEqual(BOARD_COLUMNS);
    }
  });

  it("is not named with one repository picked, however many are served", () => {
    expect(repositoryOf(job, two, false)).toBeUndefined();
    expect(columnsFor([], two, false)).toEqual(BOARD_COLUMNS);
  });

  it("comes before Tasks, so a row without a plan leaves only the trailing cell blank", () => {
    const planned = { tasks: { done: 1, working: 0, open: 1, dropped: 0 } } as JobSummary;
    expect(columnsFor([planned, job], two, true)).toEqual([...BOARD_COLUMNS, REPOSITORY_COLUMN, TASKS_COLUMN]);
  });
});

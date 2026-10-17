// What a repository is called in the picker — #886.

import { describe, expect, it } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { manifestLabel, repositoryLabel } from "@armada/shell/src/repository-label";

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

  it("names a repository by the label of the Manifest it carries, and a bare id where that Manifest is gone", () => {
    const every = [setUp("armada", "/Users/user/armada")];
    expect(manifestLabel(MINTED, every)).toBe("armada");
    // The record keeps its last known name and the wire does not carry one for a
    // Manifest nobody serves, so the id is the only honest thing left to draw.
    expect(manifestLabel("01M1GONE00000000000000000000", every)).toBe("01M1GONE00000000000000000000");
  });
});

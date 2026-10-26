// Which served repository a pasted link belongs to: one a pull request already told the forge's name of, else the Manifest's directory.
// A link to a repository Fleet does not serve is refused, never given to the only one there is.

import type { RepositorySummary } from "@armada/protocol";
import { expect, test } from "vitest";

import { manifestOfLink } from "./link";

const repository = (id: string, directory: string): RepositorySummary => ({ root: `/work/${directory}`, records_root: `/records/${directory}`, manifest: { id, repository: directory, path: `/work/${directory}/armada.yml`, records_root: `/records/${directory}`, version: 1, checks: [] } });

const LINK = { owner: "Mele-Labs", name: "armada", number: 7 };

test("a repository a listed pull request named by its forge name is that one, whatever its directory is called", () => {
  const repositories = [repository("m1", "checkout-two")];
  expect(manifestOfLink(repositories, [{ repository: "mele-labs/armada", manifestId: "m1" }], LINK)).toBe("m1");
});

test("without a listed pull request the Manifest's directory is the name, as `owner/name` or bare, in any case", () => {
  expect(manifestOfLink([repository("m1", "Armada")], [], LINK)).toBe("m1");
  expect(manifestOfLink([repository("m1", "Mele-Labs/armada")], [], LINK)).toBe("m1");
});

test("a link to a repository nothing names is refused even when only one is served", () => {
  expect(manifestOfLink([repository("m1", "storefront")], [], LINK)).toBeUndefined();
  expect(manifestOfLink([], [], LINK)).toBeUndefined();
});

test("a repository with no Manifest cannot take a review", () => {
  expect(manifestOfLink([{ root: "/work/armada", records_root: "/records/armada" }], [], LINK)).toBeUndefined();
});

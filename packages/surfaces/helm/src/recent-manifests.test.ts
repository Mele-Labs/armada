import { expect, test } from "vitest";

import type { RepositorySummary } from "@armada/protocol";
import { recentManifestIds } from "./recent-manifests";

/** A set-up repository, or one nobody set up where `setUp` is false. */
function repo(name: string, setUp = true): RepositorySummary {
  const root = `/code/${name}`;
  return {
    root,
    records_root: `/records/${name}`,
    ...(setUp
      ? { manifest: { id: `id-${name}`, repository: name, path: `${root}/armada.yml`, records_root: `/records/${name}`, version: 1, checks: [] } }
      : {}),
  };
}

const MANY = ["a", "b", "c", "d", "e", "f", "g", "h"].map((name) => repo(name));

test("past six set-up repositories the last picked lead, by Manifest id and newest first", () => {
  expect(recentManifestIds(MANY, ["/code/c", "/code/a"])).toEqual(["id-c", "id-a"]);
});

test("six or fewer get none: a Recent group would only repeat a list that fits", () => {
  expect(recentManifestIds(MANY.slice(0, 6), ["/code/a"])).toEqual([]);
});

test("a root no longer served, or never set up, is left out rather than named", () => {
  const withLoose = [...MANY, repo("loose", false)];
  expect(recentManifestIds(withLoose, ["/code/gone", "/code/loose", "/code/b"])).toEqual(["id-b"]);
});

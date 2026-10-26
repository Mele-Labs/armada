// Which served repository a pasted link belongs to: one a pull request already told the forge's name of, else the Manifest's directory.
// A link to a repository Fleet does not serve is refused, never given to the only one there is.

import type { RepositorySummary } from "@armada/protocol";
import { expect, test } from "vitest";

import { manifestOfLink, openLinked } from "./link";

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

const holding = (opened: { ok: true; key: string } | { ok: false; said: string }) => {
  const asked: { manifestId: string; number: number }[] = [];
  const store = { get: () => ({ prs: [] }), openLink: async (manifestId: string, link: { number: number }) => (asked.push({ manifestId, number: link.number }), opened) };
  return { store, asked };
};

test("a link from outside opens the review of the pull request it names, in the repository that has it", async () => {
  const { store, asked } = holding({ ok: true, key: "Mele-Labs/armada#7" });
  expect(await openLinked(store, [repository("m1", "armada")], LINK)).toBeUndefined();
  expect(asked).toEqual([{ manifestId: "m1", number: 7 }]);
});

test("a link from outside to a repository nothing names says so and asks the forge for nothing", async () => {
  const { store, asked } = holding({ ok: true, key: "x" });
  expect(await openLinked(store, [repository("m1", "storefront")], LINK)).toBe("Armada does not serve Mele-Labs/armada.");
  expect(asked).toEqual([]);
});

test("a link from outside that the store would not open says why in the store's words", async () => {
  const { store } = holding({ ok: false, said: "No such pull request" });
  expect(await openLinked(store, [repository("m1", "armada")], LINK)).toBe("No such pull request");
});

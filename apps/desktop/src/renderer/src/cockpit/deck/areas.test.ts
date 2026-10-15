// The areas the deck draws a repository in, from what Fleet reads of its base: its directories to
// two names deep, each with its files. A repository Fleet named no areas for has none.

import type { RepositoryAreas } from "@armada/protocol";
import { expect, test } from "vitest";

import { areasFromFleet, noAreas } from "./areas";

const READ: RepositoryAreas = {
  repositories: [
    {
      manifest_id: "armada",
      root: "/Users/user/armada",
      head: "abc",
      areas: [
        { dir: "crates", files: 7, kids: [{ name: "fleet", files: 4 }, { name: "ipc", files: 3 }] },
        { dir: "docs", files: 2 },
      ],
      omitted: 0,
    },
  ],
};

test("a repository's areas are its directories with the files of each, as the deck's plots take them", () => {
  expect(areasFromFleet(READ)("/Users/user/armada")).toEqual([
    { dir: "crates", kids: [["fleet", 4], ["ipc", 3]] },
    { dir: "docs", kids: [] },
  ]);
});

test("a root is found with or without its trailing slash", () => {
  expect(areasFromFleet(READ)("/Users/user/armada/")).toHaveLength(2);
});

test("a repository Fleet named no areas for, and an answer with none, draw no plots", () => {
  expect(areasFromFleet(READ)("/Users/user/other")).toEqual([]);
  expect(areasFromFleet({ repositories: [] })("/Users/user/armada")).toEqual([]);
  expect(noAreas("/Users/user/armada")).toEqual([]);
});

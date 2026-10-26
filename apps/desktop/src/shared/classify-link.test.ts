// What a link is, for a paste or a drop: a pull request, an issue, or anything else. Pure.

import { expect, test } from "vitest";

import { classifyLink } from "./classify-link";

test("a pull request's address is a pull, whatever tab it names", () => {
  expect(classifyLink("https://forge.example/Mele-Labs/armada/pull/2899/files")).toEqual({ kind: "pull", link: { owner: "Mele-Labs", name: "armada", number: 2899 } });
});

test("the deep link of a pull request is a pull", () => {
  expect(classifyLink("armada://pull/Mele-Labs/armada/2899")).toEqual({ kind: "pull", link: { owner: "Mele-Labs", name: "armada", number: 2899 } });
});

test("a forge issue is an issue with its number, repository and address", () => {
  expect(classifyLink(" https://forge.example/Mele-Labs/armada/issues/2214#issuecomment-1 ")).toEqual({
    kind: "issue",
    number: "2214",
    owner: "Mele-Labs",
    name: "armada",
    address: "https://forge.example/Mele-Labs/armada/issues/2214#issuecomment-1",
  });
});

test("a tracker's browse address is an issue by its key", () => {
  expect(classifyLink("https://tracker.example/browse/SHIP-42")).toEqual({ kind: "issue", number: "SHIP-42", address: "https://tracker.example/browse/SHIP-42" });
});

test("anything else is other: a page, a bare number, a browse path with no key, words", () => {
  for (const text of ["https://impeccable.style/", "https://forge.example/Mele-Labs/armada/issues", "https://forge.example/Mele-Labs/armada/issues/abc", "https://tracker.example/browse/ship-42", "https://tracker.example/browse/", "version = 1", ""]) {
    expect(classifyLink(text)).toEqual({ kind: "other" });
  }
});

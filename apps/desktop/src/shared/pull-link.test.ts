// A pull request named by a link: the forge's own address and Armada's `armada://pull/<owner>/<name>/<number>` read as the same
// pull request, and anything else is no pull request. Pure, since Bridge's main process and its renderer both read it.

import { expect, test } from "vitest";

import { DEEP_LINK_SCHEME, deepLinkOf, parsePullLink } from "./pull-link";

const ARMADA = { owner: "Mele-Labs", name: "armada", number: 2899 };

test("a pull request's address on the forge names its repository and number", () => {
  expect(parsePullLink("https://github.com/Mele-Labs/armada/pull/2899")).toEqual(ARMADA);
});

test("a tab, the files page or a trailing slash on the address is still the pull request", () => {
  for (const tail of ["/files", "/commits", "/", "?diff=split", "#issuecomment-1"]) {
    expect(parsePullLink(`https://github.com/Mele-Labs/armada/pull/2899${tail}`)).toEqual(ARMADA);
  }
});

test("the deep link and the forge's address read as the same pull request", () => {
  expect(parsePullLink("armada://pull/Mele-Labs/armada/2899")).toEqual(parsePullLink("https://github.com/Mele-Labs/armada/pull/2899"));
});

test("the deep link made for a pull request reads back as it", () => {
  expect(parsePullLink(deepLinkOf(ARMADA))).toEqual(ARMADA);
  expect(deepLinkOf(ARMADA).startsWith(`${DEEP_LINK_SCHEME}://`)).toBe(true);
});

test("surrounding spaces from a paste are ignored", () => {
  expect(parsePullLink("  https://github.com/Mele-Labs/armada/pull/2899\n")).toEqual(ARMADA);
});

test("a link that is not a pull request is refused", () => {
  for (const text of [
    "",
    "armada",
    "https://github.com/Mele-Labs/armada",
    "https://github.com/Mele-Labs/armada/issues/2899",
    "https://github.com/Mele-Labs/armada/pull/abc",
    "https://github.com/Mele-Labs/armada/pull/0",
    "armada://pull/Mele-Labs/2899",
    "armada://issue/Mele-Labs/armada/2899",
    "javascript:alert(1)",
    "file:///Mele-Labs/armada/pull/2899",
  ]) {
    expect(parsePullLink(text), text).toBeUndefined();
  }
});

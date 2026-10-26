// `armada://pull/<owner>/<name>/<n>`: claimed from the OS and followed while `features.deepLinks` is on, released and ignored while it is off.
// A link that arrives before Fleet has answered with the settings is held, since the person's switch is not known yet.

import { expect, test } from "vitest";

import type { PullLink } from "../shared/pull-link";
import { DeepLinks } from "./deep-link";

function links() {
  const said: string[] = [];
  const delivered: PullLink[] = [];
  const deep = new DeepLinks({ claim: () => said.push("claim"), release: () => said.push("release"), deliver: (link) => delivered.push(link) });
  return { deep, said, delivered };
}

const LINK = "armada://pull/Mele-Labs/armada/2899";
const PULL = { owner: "Mele-Labs", name: "armada", number: 2899 };

test("while the setting is on, Armada claims the scheme and a link is handed to the window", () => {
  const { deep, said, delivered } = links();
  deep.follow(true);
  expect(said).toEqual(["claim"]);
  expect(deep.arrive(LINK)).toBe(true);
  expect(delivered).toEqual([PULL]);
});

test("while the setting is off, Armada lets go of the scheme and ignores a link", () => {
  const { deep, said, delivered } = links();
  deep.follow(true);
  deep.follow(false);
  expect(said).toEqual(["claim", "release"]);
  expect(deep.arrive(LINK)).toBe(false);
  expect(delivered).toEqual([]);
});

test("turning it back on claims the scheme again, and a repeated reading of the same value claims nothing twice", () => {
  const { deep, said } = links();
  deep.follow(false);
  deep.follow(false);
  deep.follow(true);
  deep.follow(true);
  expect(said).toEqual(["release", "claim"]);
});

test("a link that arrives before the settings are read waits, and is followed or dropped by what they say", () => {
  const followed = links();
  expect(followed.deep.arrive(LINK)).toBe(true);
  expect(followed.delivered).toEqual([]);
  followed.deep.follow(true);
  expect(followed.delivered).toEqual([PULL]);

  const dropped = links();
  dropped.deep.arrive(LINK);
  dropped.deep.follow(false);
  expect(dropped.delivered).toEqual([]);
});

test("an address that is not Armada's link to a pull request is ignored, whatever the setting", () => {
  const { deep, delivered } = links();
  deep.follow(true);
  for (const address of ["armada://issue/Mele-Labs/armada/1", "armada://pull/Mele-Labs/armada", "https://github.com/Mele-Labs/armada/pull/1"]) {
    expect(deep.arrive(address), address).toBe(false);
  }
  expect(delivered).toEqual([]);
});

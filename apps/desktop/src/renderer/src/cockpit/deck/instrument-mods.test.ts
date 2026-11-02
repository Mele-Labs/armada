// A seat held on a Mod Job follows the mod it makes, and nothing else moves one.

import { expect, test } from "vitest";

import { seatsFollowing } from "./instrument-mods";

const made = {
  name: "main-build",
  kind: "instrument" as const,
  made_by: "s-1",
};

test("a seat on a Job becomes the mod that Job made, the moment Fleet lists it", () => {
  expect(
    seatsFollowing(
      [
        "beacon",
        "job:s-1",
        null,
        "mod:reviews",
      ],
      [
        made,
      ],
    ),
  ).toEqual([
    "beacon",
    "mod:main-build",
    null,
    "mod:reviews",
  ]);
});

test("with nothing to follow, the same list comes back, so nothing is written", () => {
  const seats = [
    "beacon",
    "job:s-1",
    null,
  ];
  expect(seatsFollowing(seats, [])).toBe(seats);
  expect(
    seatsFollowing(seats, [
      {
        ...made,
        made_by: "s-2",
      },
    ]),
  ).toBe(seats);
  // A tab the same Job made is no instrument.
  expect(
    seatsFollowing(seats, [
      {
        ...made,
        kind: "tab",
      },
    ]),
  ).toBe(seats);
});

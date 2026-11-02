// A drone in several write clashes carries one clash mark over it, not a row of them.

import { expect, test } from "vitest";

import { DRONES, sceneFor } from "./scene-fixture";

test("a drone in three write clashes is drawn with one clash mark", () => {
  const key = DRONES[0]!.key;
  const scene = sceneFor({
    clashed: new Map([
      [
        key,
        3,
      ],
    ]),
  });
  expect(scene.drones.find((one) => one.drone.key === key)?.clashes).toBe(1);
});

test("a drone in no write clash is drawn with none", () => {
  const scene = sceneFor();
  expect(scene.drones.every((one) => one.clashes === 0)).toBe(true);
});

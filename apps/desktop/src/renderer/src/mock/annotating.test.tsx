// The mock's layer opens as its scenario says, whatever the window held from the walk before.

import { afterEach, expect, test } from "vitest";

import { ON_KEY } from "../annotate/Layer";
import { mountAnnotating } from "./annotating";

let down: (() => void) | undefined;

afterEach(() => {
  down?.();
  sessionStorage.removeItem(ON_KEY);
});

test("a scenario whose layer starts off opens it off in a window that left it on", () => {
  sessionStorage.setItem(ON_KEY, "1");
  down = mountAnnotating("report-from-right-click");
  expect(sessionStorage.getItem(ON_KEY)).toBe("0");
});

test("a scenario with notes opens its layer on", () => {
  sessionStorage.setItem(ON_KEY, "0");
  down = mountAnnotating("annotate-to-session");
  expect(sessionStorage.getItem(ON_KEY)).toBe("1");
});

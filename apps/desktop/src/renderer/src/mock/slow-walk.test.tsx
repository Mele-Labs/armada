// A walk that is slow to reach the Job its Drone works on still finds the Drone's fix held. The mock
// Fleet answers the Drone a few seconds after it is seen, so a walker that spends them on another page
// (a person's pace in the live mock, `?walk=aDroneTrigger`) used to be answered into nothing and
// stopped on "Where the fix goes" for good.

import { expect, test } from "vitest";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { walkThrough } from "./walk";
import { WALKS } from "./walks";

unmountAfterEach();

test("the walk aDroneTrigger finds the fix held when the Trigger's Job is opened late", async () => {
  const script = WALKS.get("aDroneTrigger")!;
  const before = script.steps.findIndex((step) => "look" in step && step.say.startsWith("On a Job it fired for"));
  expect(before).toBeGreaterThan(0);
  mount(script.scenario);
  await onScreen();
  await walkThrough(script.steps.slice(0, before - 2));
  await new Promise((resolve) => setTimeout(resolve, 4_500));
  await walkThrough(script.steps.slice(before - 2));
}, 40_000);

// A walk handed over in the page's address, as `show_window` hands Bridge its `steps`: the protocol's `WalkStep`s as JSON,
// which are the mock walks' own shape without the words a stop says of its target. This is how the mock's stand-in for the
// walk window (`@armada/jobs/fake`, `openWalkWindow`) walks a page it was not given a named walk for.

import type { WalkStep, WalkTarget } from "@armada/protocol";

import type { Step, Target, Walk } from "./walk";

const targetOf = (one: WalkTarget): Target => ({
  role: one.role,
  ...(one.name === undefined ? {} : { name: one.name }),
  ...(one.exact === undefined ? {} : { exact: one.exact }),
  ...(one.within === undefined ? {} : { within: targetOf(one.within) }),
  said: one.name === undefined ? `a ${one.role}` : `a ${one.role} named “${one.name}”`,
});

/** A walk over `scenario` from the steps in the page's address, or nothing where there are none to read. */
export function inlineWalk(steps: string | null, scenario: string): Walk | undefined {
  if (steps === null) return undefined;
  try {
    const read = JSON.parse(steps) as WalkStep[];
    const made = read.map((one): Step => {
      if ("press" in one) return { ...one, press: targetOf(one.press) };
      if ("look" in one) return { ...one, look: targetOf(one.look) };
      if ("hover" in one) return { ...one, hover: targetOf(one.hover) };
      if ("later" in one) return { ...one, later: targetOf(one.later) };
      if ("type" in one) return { ...one, into: targetOf(one.into) };
      if ("key" in one) return { ...one, on: targetOf(one.on) };
      if ("select" in one) return { ...one, in: targetOf(one.in) };
      if ("paste" in one) return { ...one, paste: targetOf(one.paste) };
      return { ...one, drag: targetOf(one.drag) };
    });
    return made.length === 0 ? undefined : { scenario, steps: made };
  } catch {
    return undefined;
  }
}

// What a sketch's `mock` is drawn with on the phone: the style set the Mac kept for it, read through the Gateway by the hash the
// node carries. A hash is only known to the Mac that kept it, so each Mac reads and remembers its own.

import type { ReactNode } from "react";
import { MockStylesContext, type ResolveMockStyles } from "@armada/components";
import { createKeptStyles } from "@armada/screens/src/kept-styles";

import { readKept } from "./jobState";
import type { Machine } from "./machines";

const held = new Map<string, ResolveMockStyles>();

function stylesOf(machine: Machine): ResolveMockStyles {
  const had = held.get(machine.origin);
  if (had !== undefined) return had;
  const made = createKeptStyles((hash) => readKept(machine, hash), () => document.documentElement.getAttribute("data-theme"));
  held.set(machine.origin, made);
  return made;
}

export function WithKeptStyles({ machine, children }: { machine: Machine | undefined; children: ReactNode }) {
  return <MockStylesContext.Provider value={machine === undefined ? undefined : stylesOf(machine)}>{children}</MockStylesContext.Provider>;
}

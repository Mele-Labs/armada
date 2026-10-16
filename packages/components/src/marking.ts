// An element that says, on an ancestor, that it is there: a `data-` attribute the ancestor's CSS reads.
//
// **Why not `:has()`.** A rule whose subject is a high container (`.armada-shell`, `body`, `:root`) is restyled by the
// engine whenever anything is inserted anywhere beneath it, matching or not: a measured 345 to 390 restyles of each of
// html, body, the shell and its work area in 25 s of a live Cockpit. The element that is the reason sets the mark itself, once, where it
// mounts, and the ancestor is touched only when it comes or goes.

import { useCallback, useRef } from "react";

export type Mark = {
  /** The ancestor, by what `closest` takes. */
  of: string;
  /** The attribute it is given, empty. */
  as: string;
  /** Only the element's own parent, where the rule said `> `. */
  parent?: boolean;
};

/** How many of the elements marking each ancestor are there, so one leaving does not take a mark the others still stand on. */
const COUNTS = new WeakMap<Element, Map<string, number>>();

function bump(target: Element, name: string, by: 1 | -1): void {
  const counts = COUNTS.get(target) ?? new Map<string, number>();
  COUNTS.set(target, counts);
  const next = (counts.get(name) ?? 0) + by;
  counts.set(name, next);
  if (next > 0) {
    if (!target.hasAttribute(name)) target.setAttribute(name, "");
  } else target.removeAttribute(name);
}

/** A `ref` that marks the ancestors `marks` name for as long as the element is mounted. Pass a constant list. */
export function useMarking(marks: readonly Mark[]): (element: Element | null) => void {
  const held = useRef<{ target: Element; name: string }[]>([]);
  return useCallback(
    (element: Element | null) => {
      for (const one of held.current) bump(one.target, one.name, -1);
      held.current = [];
      if (element === null) return;
      for (const mark of marks) {
        const target = mark.parent === true ? (element.parentElement?.matches(mark.of) ? element.parentElement : null) : element.parentElement?.closest(mark.of);
        if (target === null || target === undefined) continue;
        bump(target, mark.as, 1);
        held.current.push({ target, name: mark.as });
      }
    },
    [marks],
  );
}

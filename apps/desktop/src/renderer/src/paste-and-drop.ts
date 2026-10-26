// A paste or a drop anywhere in the window, heard once, at the shell, and handed to whichever claims it: a package
// first (`sharing/ImportPackage.tsx`), then a pull request's or an issue's link (`link-routes.ts`), then any other link
// (`link-drop.tsx`). **The order is this file's**, not the order they happen to mount in, so none has to know the others are listening.

import { useEffect, useRef } from "react";

/** One kind of thing a paste or a drop can be. Each answers whether it took the event; the first that does ends it. */
export type Claimant = {
  paste?: (event: ClipboardEvent) => boolean;
  over?: (event: DragEvent) => boolean;
  drop?: (event: DragEvent) => boolean;
};

const ORDER = ["package", "pull", "issue", "link"] as const;
type ClaimantName = (typeof ORDER)[number];

const claimants = new Map<ClaimantName, Claimant>();

function handOut<E extends Event>(event: E, each: (one: Claimant) => ((event: E) => boolean) | undefined): void {
  // A field or a component that took it first keeps it.
  if (event.defaultPrevented) return;
  for (const name of ORDER) {
    const claim = claimants.get(name);
    if (claim !== undefined && each(claim)?.(event) === true) return;
  }
}

/** The one listener. Mounted once, by `App`. */
export function usePasteAndDrop(): void {
  useEffect(() => {
    const paste = (event: ClipboardEvent) => handOut(event, (one) => one.paste);
    const over = (event: DragEvent) => handOut(event, (one) => one.over);
    const drop = (event: DragEvent) => handOut(event, (one) => one.drop);
    window.addEventListener("paste", paste);
    window.addEventListener("dragover", over);
    window.addEventListener("drop", drop);
    return () => {
      window.removeEventListener("paste", paste);
      window.removeEventListener("dragover", over);
      window.removeEventListener("drop", drop);
    };
  }, []);
}

/** Offers `name` what the shell hears, while the caller is mounted. The latest `claimant` answers, so it may close over state. */
export function useClaim(name: ClaimantName, claimant: Claimant): void {
  const latest = useRef(claimant);
  latest.current = claimant;
  useEffect(() => {
    claimants.set(name, {
      paste: (event) => latest.current.paste?.(event) ?? false,
      over: (event) => latest.current.over?.(event) ?? false,
      drop: (event) => latest.current.drop?.(event) ?? false,
    });
    return () => {
      claimants.delete(name);
    };
  }, [name]);
}

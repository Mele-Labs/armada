// Time passing in a mock Fleet: the next moment a scenario holds, published when a walk's step says
// so. A scenario that names `later` has the window's fake register here, and a walk's `later` step
// calls it, so the link, the capture and the test all move through a turn the same way. A scenario
// with no `later` registers nothing, and a `later` step then does nothing.

let next: (() => void) | undefined;

/** The fake a scenario's window was built with takes the next moment. The latest window wins. */
export function onTimePassing(advance: (() => void) | undefined): void {
  next = advance;
}

/** The scenario's next moment, if it holds one. */
export function timePasses(): void {
  next?.();
}

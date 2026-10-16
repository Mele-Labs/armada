// Which take of the Activity chart the window draws, where the mock's scenario says so. The app's own window holds none, and draws the stream.

let held: string | undefined;

/** Set at each mount, so a window never inherits the take the last one drew. */
export function holdChart(take: string | undefined): void {
  held = take;
}

export const heldChart = (): string | undefined => held;

/** One frame: the renderer cannot draw more often than this, so a send between frames is never seen. */
export const FRAME_MS = 16;

/** `send` the latest value at most once a frame, however many arrive inside it. */
export function coalesce<T>(send: (latest: T) => void): (value: T) => void {
  let latest: T;
  let scheduled = false;
  return (value) => {
    latest = value;
    if (scheduled) return;
    scheduled = true;
    setTimeout(() => {
      scheduled = false;
      send(latest);
    }, FRAME_MS);
  };
}

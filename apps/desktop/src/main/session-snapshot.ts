// The live view's picture of a page. A view over the panel cannot be scaled, so main loads the page out of sight and takes a picture of it
// when it loads or moves within itself and every few seconds, and sends the renderer the bytes only when they differ from the last.

/** What a picture is taken of: the page, and what tells it moved. */
export type SnapshotSource = {
  /** A PNG of the page as it reads now, or `undefined` where none could be taken. */
  take: () => Promise<Uint8Array | undefined>;
  /** Hears the page finish loading or change its address within itself. */
  onChange: (on: () => void) => void;
};

/** How often a picture is taken of a page that gives no sign it moved: an animation, a timer, a request that came back. */
export const SNAPSHOT_EVERY_MS = 3000;

const same = (a: Uint8Array | undefined, b: Uint8Array): boolean => a !== undefined && a.byteLength === b.byteLength && a.every((byte, at) => byte === b[at]);

/** Starts pictures of `source`, handing each new one to `send`. Returns what ends them. **One is taken at a time**: a change during one asks for one more after it. */
export function keepSnapshots(source: SnapshotSource, send: (png: Uint8Array) => void, every: number = SNAPSHOT_EVERY_MS): () => void {
  let stopped = false;
  let taking = false;
  let again = false;
  let last: Uint8Array | undefined;
  const take = async (): Promise<void> => {
    if (stopped) return;
    if (taking) {
      again = true;
      return;
    }
    taking = true;
    try {
      const png = await source.take();
      if (!stopped && png !== undefined && !same(last, png)) {
        last = png;
        send(png);
      }
    } catch {
      // A page that went away between the ask and the picture has none to send.
    } finally {
      taking = false;
    }
    if (again && !stopped) {
      again = false;
      await take();
    }
  };
  source.onChange(() => void take());
  const timer = setInterval(() => void take(), every);
  return () => {
    stopped = true;
    clearInterval(timer);
  };
}

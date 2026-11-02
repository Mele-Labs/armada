// A value copied from deep in a surface, announced to the window's own copied toast, which only the root draws.

const heard = new Set<(value: string) => void>();

/** Hears each value announced as copied: the root, which shows the toast. */
export function onCopiedAnnounced(listener: (value: string) => void): () => void {
  heard.add(listener);
  return () => void heard.delete(listener);
}

/** Puts `value` on the clipboard and says so. */
export function copyAnnounced(value: string): void {
  void navigator.clipboard?.writeText(value);
  heard.forEach((one) => one(value));
}

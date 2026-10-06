// A file's own name off its path. Kept apart from `editing.ts`, the Manifest
// surface's, so the draft ledger can name a file without importing it.

/**
 * The file's own name, off the path Fleet resolved — what the toggle is
 * called. **Never the format**: the lexicon bans naming a Manifest as one.
 */
export function fileNameOf(path: string): string {
  const parts = path.split("/").filter((part) => part !== "");
  return parts[parts.length - 1] ?? path;
}

// The components' story files, for the note card's Variant tool to draw each story over its name. Lazy, so a story
// file's chunk loads only when the tool opens; the keys are made the index's, repo-relative paths.

import type { StoryModule } from "../annotate/story";

declare global {
  interface ImportMeta {
    /** Vite's, read at build time, lazy: each file is a function that loads it. `walks.ts` declares the eager one. */
    glob<T>(pattern: string): Record<string, () => Promise<T>>;
  }
}

const FILES = import.meta.glob<StoryModule>(
  "../../../../../../packages/components/src/**/*.stories.tsx",
);

/** `file` is the index's path from the repository root. Null for a file the glob does not hold. */
export async function storyFile(file: string): Promise<StoryModule | null> {
  const load = Object.entries(FILES).find(([key]) => key.replace(/^(\.\.\/)+/, "") === file)?.[1];
  return load === undefined ? null : load();
}

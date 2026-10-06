// Every walk the mock can play, by the name each is exported under: the
// checked-in ones in `walks/`, and an agent's scratch ones in `walks/scratch/`,
// which git ignores. A file added to either is a walk with no other edit.

import type { Walk } from "./walk";

declare global {
  interface ImportMeta {
    /** Vite's, read at build time. The renderer's tsconfig takes no `vite/client`. */
    glob<T>(pattern: string, options: { eager: true }): Record<string, T>;
  }
}

/** Each walk a set of files exports, by its export name. Two of one name is a mistake, said at once. */
function named(files: Record<string, Record<string, Walk>>, into = new Map<string, Walk>()): Map<string, Walk> {
  for (const [file, exported] of Object.entries(files)) {
    for (const [name, one] of Object.entries(exported)) {
      if (into.has(name)) throw new Error(`two walks are named ${name}; the second is in ${file}`);
      into.set(name, one);
    }
  }
  return into;
}

/** The walks in `walks/`. Each runs as a test in one of the `walks-*of4.test.tsx` files. */
export const WALKS: ReadonlyMap<string, Walk> = named(import.meta.glob<Record<string, Walk>>("./walks/*.ts", { eager: true }));

/** Every walk the mock plays: the checked-in ones and the scratch ones. */
export const EVERY_WALK: ReadonlyMap<string, Walk> = named(
  import.meta.glob<Record<string, Walk>>("./walks/scratch/*.ts", { eager: true }),
  new Map(WALKS),
);

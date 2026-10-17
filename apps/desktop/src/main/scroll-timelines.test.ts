// A rule that drives an animation by a scroll or view timeline sets it with longhands. Written with the `animation` shorthand, the build's
// lightningcss folds the `animation-timeline` after it into that shorthand (`animation: linear both name --timeline`), Chromium refuses the
// declaration, and nothing moves. The mock's dev server does not minify, so a walk plays it and the built Bridge does not: the calls pinned
// under a Session's thread never shrank on 16 Oct 2026.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { expect, test } from "vitest";

const ROOT = new URL("../../../../", import.meta.url).pathname;
const SOURCES = ["apps/desktop/src", "packages"];
const SKIPPED = new Set(["node_modules", "out", "dist", "storybook-static", "target"]);

function cssFiles(dir: string, found: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    if (SKIPPED.has(name)) continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) cssFiles(path, found);
    else if (name.endsWith(".css")) found.push(path);
  }
  return found;
}

/** The innermost blocks of a sheet that set both the `animation` shorthand and `animation-timeline`. */
export function foldable(css: string): string[] {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  return [...text.matchAll(/([^{};]*)\{([^{}]*)\}/g)]
    .filter((block) => /(^|[;\s])animation\s*:/.test(block[2]!) && /animation-timeline\s*:/.test(block[2]!))
    .map((block) => block[1]!.trim());
}

test("a rule with an animation timeline names its animation with longhands", () => {
  const refused = SOURCES.flatMap((one) => cssFiles(join(ROOT, one))).flatMap((file) => foldable(readFileSync(file, "utf8")).map((rule) => `${relative(ROOT, file)}: ${rule}`));
  expect(refused).toEqual([]);
});

test("the shorthand beside a timeline is found, and longhands are not", () => {
  expect(foldable(".a { animation: shrink linear both; animation-timeline: --scroll; }")).toEqual([".a"]);
  expect(foldable(".a { animation-name: shrink; animation-fill-mode: both; animation-timeline: --scroll; }")).toEqual([]);
  expect(foldable(".a { animation: spin 1s infinite; }")).toEqual([]);
});

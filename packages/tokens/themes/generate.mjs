// Generates the theme catalogue: `css/<slug>.css` per theme and `index.json`, from Ghostty
// colour-scheme files. docs/concepts/themes.md says where the schemes come from and under what
// licence.
//
//   node packages/tokens/themes/generate.mjs           write css/ and index.json from ghostty/ and popular.txt
//   node packages/tokens/themes/generate.mjs --check   fail if what is checked in is not what that writes
//   node packages/tokens/themes/generate.mjs --all <dir>   every scheme in <dir> into more/ (not checked in)
//
// The rest of the collection: `git clone --depth 1 --filter=blob:none --sparse
// https://github.com/mbadolato/iTerm2-Color-Schemes`, then `git sparse-checkout set ghostty`, and
// pass its `ghostty/` directory. `more/` is ignored by git and read by Bridge in the same way.

import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { derive, idOf, parseScheme, slugOf, titleOf } from "./derive.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const SOURCE = "mbadolato/iTerm2-Color-Schemes";

/** Every output of one run: relative path to contents. */
export function build(schemes) {
  const files = new Map();
  const index = [];
  const problems = [];
  for (const [name, text] of schemes) {
    const made = derive(idOf(name), titleOf(name), parseScheme(text));
    if (made.flags.length > 0) problems.push(`${name}: ${made.flags.join("; ")}`);
    files.set(`css/${slugOf(name)}.css`, made.css);
    index.push({ id: idOf(name), title: titleOf(name), tone: made.tone, source: name, flags: made.flags });
  }
  index.sort((a, b) => a.title.localeCompare(b.title));
  files.set("index.json", JSON.stringify(index, null, 2) + "\n");
  return { files, problems };
}

export function popular() {
  const names = readFileSync(join(here, "popular.txt"), "utf8").split("\n").map((l) => l.trim()).filter((l) => l !== "" && !l.startsWith("#"));
  return names.map((name) => [name, readFileSync(join(here, "ghostty", name), "utf8")]);
}

const args = process.argv.slice(2);
if (import.meta.url === `file://${process.argv[1]}`) {
  if (args[0] === "--all") {
    const dir = args[1];
    if (dir === undefined || !existsSync(dir)) throw new Error("--all needs the ghostty/ directory of a checkout of " + SOURCE);
    const schemes = readdirSync(dir).filter((n) => !n.startsWith(".") && n !== "LICENSE").map((n) => [n, readFileSync(join(dir, n), "utf8")]);
    const { files, problems } = build(schemes.filter(([, text]) => { try { parseScheme(text); return true; } catch { return false; } }));
    rmSync(join(here, "more"), { recursive: true, force: true });
    for (const [path, text] of files) {
      mkdirSync(dirname(join(here, "more", path)), { recursive: true });
      writeFileSync(join(here, "more", path), text);
    }
    console.log(`wrote ${files.size - 1} themes to more/, ${problems.length} flagged (unreadable, shipped with their flags)`);
  } else {
    const { files, problems } = build(popular());
    if (problems.length > 0) {
      console.error("not shipped, fix the scheme or drop it from popular.txt:\n" + problems.map((p) => "  " + p).join("\n"));
      process.exit(1);
    }
    if (args[0] === "--check") {
      const stale = [...files].filter(([path, text]) => !existsSync(join(here, path)) || readFileSync(join(here, path), "utf8") !== text).map(([path]) => path);
      const extra = existsSync(join(here, "css")) ? readdirSync(join(here, "css")).filter((n) => !files.has(`css/${n}`)) : [];
      if (stale.length + extra.length > 0) {
        console.error("themes are stale; run generate.mjs:\n" + [...stale, ...extra.map((n) => `css/${n} (no longer in popular.txt)`)].map((p) => "  " + p).join("\n"));
        process.exit(1);
      }
      console.log(`themes: ${files.size - 1} current`);
    } else {
      rmSync(join(here, "css"), { recursive: true, force: true });
      for (const [path, text] of files) {
        mkdirSync(dirname(join(here, path)), { recursive: true });
        writeFileSync(join(here, path), text);
      }
      console.log(`wrote ${files.size - 1} themes`);
    }
  }
}

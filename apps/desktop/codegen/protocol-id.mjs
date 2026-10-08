// The protocol ID Bridge is built with: FNV-1a 64 over the wire surface.
//
// The file list, the comment-line rule and the hash are `protocol_id` in
// `crates/ipc/build.rs`, written again here, and `cargo xtask verify-foundations`
// fails when the two disagree. Comment-only lines are dropped; a comment after
// code on the same line still counts.
//
// Nothing is written to the tree. The ID is injected into Bridge's build as
// `__PROTOCOL_ID__` (`electron.vite.config.ts`), because a checked-in copy would
// be a line every branch that touches the wire edits, and two of them conflict.
//
// Run it with `node apps/desktop/codegen/protocol-id.mjs` to print the ID.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..", "..");

function wireFiles(dir, keep, into = []) {
  for (const entry of readdirSync(join(repo, dir), { withFileTypes: true })) {
    const rel = `${dir}/${entry.name}`;
    if (entry.isDirectory()) wireFiles(rel, keep, into);
    else if (keep(rel)) into.push(rel);
  }
  return into;
}

export function protocolId() {
  const files = [
    ...wireFiles(
      "crates/ipc/src",
      (rel) => rel.endsWith(".rs") && !rel.includes("/tests/") && !rel.endsWith("/tests.rs"),
    ),
    ...wireFiles("crates/api/src/routes", (rel) => rel.endsWith(".rs")),
    ...wireFiles(
      "crates/ipc/operations",
      (rel) => rel.endsWith(".toml") && !rel.endsWith("/_header.toml"),
    ),
    ...wireFiles(
      "packages/protocol/src",
      (rel) => rel.endsWith(".ts") && !rel.endsWith(".test.ts") && !rel.endsWith("/connection.ts"),
    ),
  ].sort();
  const MASK = 0xffffffffffffffffn;
  const PRIME = 0x100000001b3n;
  let hash = 0xcbf29ce484222325n;
  const feed = (bytes) => {
    for (const byte of bytes) hash = ((hash ^ BigInt(byte)) * PRIME) & MASK;
  };
  const encoder = new TextEncoder();
  for (const rel of files) {
    const comment = rel.endsWith(".toml") ? "#" : "//";
    feed(encoder.encode(`${rel}\n`));
    for (const raw of readFileSync(join(repo, rel), "utf8").split("\n")) {
      const line = raw.trimEnd();
      if (line === "" || line.trimStart().startsWith(comment)) continue;
      feed(encoder.encode(`${line}\n`));
    }
    feed([0]);
  }
  return hash.toString(16).padStart(16, "0");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) process.stdout.write(`${protocolId()}\n`);

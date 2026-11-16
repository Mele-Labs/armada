// Bundles the Worker into one module for Fleet to upload, or checks that the committed one is current.
// `pnpm bundle` writes dist/worker.js; `pnpm bundle:check` compares a fresh bundle with it byte for byte.
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = join(root, "dist", "worker.js");

function bundled() {
  const out = mkdtempSync(join(tmpdir(), "armada-store-bundle-"));
  try {
    // Wrangler's own bundler, no minify, so a diff of the module reads. `--dry-run` uploads nothing.
    execFileSync(
      "pnpm",
      [
        "exec",
        "wrangler",
        "deploy",
        "--dry-run",
        "--outdir",
        out,
      ],
      {
        cwd: root,
        stdio: [
          "ignore",
          "ignore",
          "inherit",
        ],
      },
    );
    // The map is not kept, so the comment that points at it goes too.
    const text = readFileSync(join(out, "index.js"), "utf8").replace(
      /^\/\/# sourceMappingURL=.*\n?/m,
      "",
    );
    for (const local of [
      root,
      out,
      process.env.HOME,
    ]) {
      if (local && text.includes(local)) throw new Error(`the bundle names a local path, ${local}`);
    }
    return text;
  } finally {
    rmSync(out, {
      recursive: true,
      force: true,
    });
  }
}

const text = bundled();
if (process.argv.includes("--check")) {
  let current = null;
  try {
    current = readFileSync(target, "utf8");
  } catch {}
  if (current !== text) {
    console.error("apps/store/dist/worker.js is stale: run pnpm --dir apps/store bundle");
    process.exit(1);
  }
} else {
  mkdirSync(dirname(target), {
    recursive: true,
  });
  writeFileSync(target, text);
}

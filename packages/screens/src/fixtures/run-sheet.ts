// The run sheet a mock Fleet answers `get_checkout_run_sheet` with. Shared by the Manifest and
// Studio fleets, so neither imports the other.

import type { CheckoutRunSheet, RunEntry, ServerEntry } from "@armada/protocol";

/** One row, in the shape `RunEntry` has with nothing narrowed against it. */
function entry(
  name: string,
  run: string,
  extra: Partial<RunEntry> = {},
): RunEntry {
  return {
    name,
    run,
    narrows: false,
    requires: [],
    expect_exit_code: 0,
    destructive: false,
    // Nothing in the main checkout is frozen: this is the file Fleet holds.
    frozen: false,
    ...extra,
  };
}

const STORYBOOK: ServerEntry = {
  name: "storybook_dev",
  serve: "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci",
  ready: "curl -sf http://localhost:41207",
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  destructive: false,
};

/** This repository's Manifest, as `get_checkout_run_sheet` answers it. */
export function sheet(over: Partial<CheckoutRunSheet> = {}): CheckoutRunSheet {
  return {
    setup: [
      entry("bootstrap", "pnpm install --frozen-lockfile"),
      entry(
        "browsers",
        "pnpm -C packages/components exec playwright install chromium --only-shell",
      ),
    ],
    checks: [
      entry("build", "cargo build --workspace --locked", { narrows: true }),
      entry("test", "cargo nextest run --workspace --exclude acceptance", { narrows: true }),
      entry("typecheck", "pnpm typecheck"),
      entry("bridge_build", "pnpm -C apps/desktop build"),
      entry("storybook", "pnpm -C packages/components build-storybook"),
      entry("bridge_test", "pnpm bridge-test"),
      entry("format", "cargo fmt --all --check", { narrows: true }),
    ],
    commands: [
      entry("fmt", "cargo fmt --all", { destructive: true }),
      entry("gate", "cargo xtask verify-foundations", { expect_exit_code: 0 }),
    ],
    manifest_edited_at: "2026-09-10T09:14:00Z",
    servers: [STORYBOOK],
    ...over,
  };
}

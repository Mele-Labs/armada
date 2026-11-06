// Reloading Bridge's page in place when the person swaps or reverts onto another kept build.
//
// **A page reload is safe only when main, preload and Electron are unchanged.** The renderer talks to
// main through the preload's channels, and a new page against old main code is sound only if both sides
// of that seam are the ones the page was built with. `bridge_shell` in `build.toml` hashes `out/main` and
// `out/preload`; the `electron` line is the runtime they run in. When both match, the new build's
// renderer can load over the running shell. When either differs, or the running build predates this
// code, Fleet relaunches Bridge instead, and nothing here runs.
//
// **No Electron import, on purpose**, as `annotations.ts` does: the decision is a pure read of two build
// folders, and main passes in what it owns.
//
// **Only the main window is reloaded.** A terminal, a browser or any other window keeps the page it
// loaded until it is reopened.

import { existsSync, readFileSync, realpathSync } from "node:fs";
import { join, resolve } from "node:path";

/**
 * Written to the log when a page is reloaded, and read by `scripts/keep-build` in the bundled
 * `out/main`: a build holding this text can be reloaded onto, and `page_reload = true` says so.
 */
export const PAGE_RELOAD_MARK = "armada:page-reload";

/** One `key = "value"` line of a `build.toml`, or null where the file or the line is absent. */
function keyOf(record: string, key: string): string | null {
  const quoted = new RegExp(`^${key} = (".*")$`, "m").exec(record)?.[1];
  if (quoted === undefined) return null;
  try {
    return JSON.parse(quoted) as string;
  } catch {
    return null;
  }
}

function recordIn(build: string): string | null {
  try {
    return readFileSync(join(build, "build.toml"), "utf8");
  } catch {
    return null;
  }
}

function realpath(path: string): string | null {
  try {
    return realpathSync(path);
  } catch {
    return null;
  }
}

/** The build folder Bridge runs from, which is the app path's parent, and the one `current` names. */
function buildsOf(
  appPath: string,
  builds: string,
): {
  own: string;
  current: string;
} | null {
  const own = realpath(join(resolve(appPath), ".."));
  const current = realpath(join(builds, "current"));
  return own === null || current === null
    ? null
    : {
        own,
        current,
      };
}

/** True when `builds/current` names the folder Bridge runs from. False for a dev tree, which has none. */
export function onCurrentBuild(appPath: string, builds: string): boolean {
  const both = buildsOf(appPath, builds);
  return both !== null && both.own === both.current && recordIn(both.own) !== null;
}

/**
 * The `index.html` to load after Fleet came back on another build, or null where the running page stays
 * (or Fleet relaunches Bridge). Every condition is the one that makes a reload safe: see the header.
 */
export function pageAfterSwap(appPath: string, builds: string): string | null {
  const both = buildsOf(appPath, builds);
  if (both === null || both.own === both.current) return null;
  const mine = recordIn(both.own);
  const theirs = recordIn(both.current);
  if (mine === null || theirs === null) return null;
  const shell = keyOf(mine, "bridge_shell");
  if (shell === null || shell === "" || shell !== keyOf(theirs, "bridge_shell")) return null;
  if (keyOf(mine, "electron") !== keyOf(theirs, "electron")) return null;
  if (keyOf(mine, "bridge") === keyOf(theirs, "bridge")) return null;
  const page = join(both.current, "bridge", "out", "renderer", "index.html");
  return existsSync(page) ? page : null;
}

export type PageReloadWiring = {
  /** `pageAfterSwap` for this Bridge, read now. */
  after: () => string | null;
  /** `onCurrentBuild` for this Bridge, read now. */
  onOwnBuild: () => boolean;
  /** Load `page` into the main window. */
  load: (page: string) => void;
  log?: (line: string) => void;
};

/**
 * Reloads the main window once per change of page, each time Fleet's connection comes up after being down.
 *
 * **Swapping back is a reload too**: when `current` names the build Bridge started on again, the page
 * it first loaded is the one to return to, since `pageAfterSwap` is null for that build.
 */
export class PageReloads {
  private up = false;
  private first: string | null = null;
  private last: string | null = null;

  constructor(private readonly wiring: PageReloadWiring) {}

  /** The main window's first load, remembered and not reloaded. */
  loaded(page: string): void {
    this.first ??= page;
    this.last = page;
  }

  /** Told on every state main publishes. Acts only on the edge from down to up. */
  connection(connected: boolean): void {
    const was = this.up;
    this.up = connected;
    if (!connected || was || this.last === null) return;
    const page = this.wiring.after() ?? (this.wiring.onOwnBuild() ? this.first : null);
    if (page === null || page === this.last) return;
    this.wiring.log?.(`${PAGE_RELOAD_MARK}: ${this.last} -> ${page}`);
    this.last = page;
    this.wiring.load(page);
  }
}

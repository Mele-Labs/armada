// The page reload after a swap, over real build folders and a real `current` symlink.

import {
  mkdirSync,
  mkdtempSync,
  realpathSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { onCurrentBuild, PAGE_RELOAD_MARK, PageReloads, pageAfterSwap } from "./page-reload";

let root: string;
let builds: string;

beforeEach(() => {
  root = realpathSync(mkdtempSync(join(tmpdir(), "page-reload-")));
  builds = join(root, "builds");
  mkdirSync(builds);
});
afterEach(() =>
  rmSync(root, {
    recursive: true,
    force: true,
  }),
);

type Record = {
  bridge?: string;
  bridge_shell?: string;
  electron?: string;
};

const SAME: Record = {
  bridge: "aaaa",
  bridge_shell: "5555",
  electron: "40.10.6",
};

/** A kept build: its record and a renderer page. */
function keep(name: string, record: Record, renderer = true): string {
  const build = join(builds, name);
  mkdirSync(join(build, "bridge", "out", "renderer"), {
    recursive: true,
  });
  const lines = Object.entries(record).map(([key, value]) => `${key} = "${value}"`);
  writeFileSync(join(build, "build.toml"), `${lines.join("\n")}\npage_reload = true\n`);
  if (renderer) writeFileSync(join(build, "bridge", "out", "renderer", "index.html"), "<html>");
  return build;
}

function current(name: string): void {
  const link = join(builds, "current");
  try {
    unlinkSync(link);
  } catch {
    // no link yet
  }
  symlinkSync(join(builds, name), link);
}

const appOf = (build: string): string => join(build, "bridge");

describe("pageAfterSwap", () => {
  it("gives null for a dev tree, which has no build.toml", () => {
    keep("new", {
      ...SAME,
      bridge: "bbbb",
    });
    current("new");
    const tree = join(root, "tree", "apps", "desktop");
    mkdirSync(tree, {
      recursive: true,
    });
    expect(pageAfterSwap(tree, builds)).toBeNull();
    expect(onCurrentBuild(tree, builds)).toBe(false);
  });

  it("gives null when current is the build Bridge runs from", () => {
    const own = keep("own", SAME);
    current("own");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
    expect(onCurrentBuild(appOf(own), builds)).toBe(true);
  });

  it("gives the new index.html when only the page changed", () => {
    const own = keep("own", SAME);
    const next = keep("next", {
      ...SAME,
      bridge: "bbbb",
    });
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBe(
      join(next, "bridge", "out", "renderer", "index.html"),
    );
  });

  it("follows the current link as it is, through a symlinked app path", () => {
    const own = keep("own", SAME);
    keep("next", {
      ...SAME,
      bridge: "bbbb",
    });
    current("next");
    const via = join(root, "via");
    symlinkSync(own, via);
    expect(pageAfterSwap(appOf(via), builds)).not.toBeNull();
  });

  it("gives null when the shell differs", () => {
    const own = keep("own", SAME);
    keep("next", {
      ...SAME,
      bridge: "bbbb",
      bridge_shell: "6666",
    });
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });

  it("gives null when either record has no shell hash", () => {
    const own = keep("own", {
      bridge: "aaaa",
      electron: "40.10.6",
    });
    keep("next", {
      bridge: "bbbb",
      electron: "40.10.6",
    });
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });

  it("gives null when Electron differs", () => {
    const own = keep("own", SAME);
    keep("next", {
      ...SAME,
      bridge: "bbbb",
      electron: "41.0.0",
    });
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });

  it("gives null when only one record names Electron", () => {
    const own = keep("own", SAME);
    keep("next", {
      bridge: "bbbb",
      bridge_shell: "5555",
    });
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });

  it("gives the page when neither record names Electron", () => {
    const own = keep("own", {
      bridge: "aaaa",
      bridge_shell: "5555",
    });
    keep("next", {
      bridge: "bbbb",
      bridge_shell: "5555",
    });
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).not.toBeNull();
  });

  it("gives null when the bridge hash is the same", () => {
    const own = keep("own", SAME);
    keep("next", SAME);
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });

  it("gives null when the new renderer is missing", () => {
    const own = keep("own", SAME);
    keep(
      "next",
      {
        ...SAME,
        bridge: "bbbb",
      },
      false,
    );
    current("next");
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });

  it("gives null when there is no current link", () => {
    const own = keep("own", SAME);
    expect(pageAfterSwap(appOf(own), builds)).toBeNull();
  });
});

describe("PageReloads", () => {
  const FIRST = "/builds/own/bridge/out/renderer/index.html";
  const NEXT = "/builds/next/bridge/out/renderer/index.html";

  function wired(after: () => string | null, onOwnBuild = () => false) {
    const load = vi.fn();
    const log = vi.fn();
    const reloads = new PageReloads({
      after,
      onOwnBuild,
      load,
      log,
    });
    reloads.loaded(FIRST);
    return {
      reloads,
      load,
      log,
    };
  }

  it("reloads once on reconnect when the page changed, and not again for the same page", () => {
    const { reloads, load, log } = wired(() => NEXT);
    reloads.connection(false);
    reloads.connection(true);
    expect(load).toHaveBeenCalledTimes(1);
    expect(load).toHaveBeenCalledWith(NEXT);
    expect(log).toHaveBeenCalledWith(expect.stringContaining(PAGE_RELOAD_MARK));
    reloads.connection(false);
    reloads.connection(true);
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("does not reload on a published state that stays connected", () => {
    const { reloads, load } = wired(() => NEXT);
    reloads.connection(false);
    reloads.connection(true);
    reloads.connection(true);
    reloads.connection(true);
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("does not reload when no page is named", () => {
    const { reloads, load } = wired(() => null);
    reloads.connection(false);
    reloads.connection(true);
    expect(load).not.toHaveBeenCalled();
  });

  it("returns to the first page when current names the running build again", () => {
    let page: string | null = NEXT;
    let own = false;
    const { reloads, load } = wired(
      () => page,
      () => own,
    );
    reloads.connection(false);
    reloads.connection(true);
    page = null;
    own = true;
    reloads.connection(false);
    reloads.connection(true);
    expect(load.mock.calls).toEqual([
      [
        NEXT,
      ],
      [
        FIRST,
      ],
    ]);
  });

  it("reloads nothing before the first load is remembered", () => {
    const load = vi.fn();
    const reloads = new PageReloads({
      after: () => NEXT,
      onOwnBuild: () => false,
      load,
    });
    reloads.connection(false);
    reloads.connection(true);
    expect(load).not.toHaveBeenCalled();
  });
});

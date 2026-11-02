// The loader for a tab mod's module. A node test has no `blob:` to import from, so the module is run from a `data:` address, which the loader takes as its importer.

import { describe, expect, it } from "vitest";

import { loadTabModule, type Importer } from "./tab-module";

const fromData: Importer = (code) =>
  import(/* @vite-ignore */ `data:text/javascript,${encodeURIComponent(code)}`);

describe("loadTabModule", () => {
  it("answers the component a module exports by default, which is handed host", async () => {
    const Tab = await loadTabModule(
      "export default function Tab({ host }) { return host.say; }",
      fromData,
    );
    expect(typeof Tab).toBe("function");
    expect(
      (
        Tab as unknown as (props: {
          host: {
            say: string;
          };
        }) => string
      )({
        host: {
          say: "hi",
        },
      }),
    ).toBe("hi");
  });

  it("answers another component each time, so a module the Job rewrote is not the one before", async () => {
    const first = await loadTabModule("export default function Tab() { return 1; }", fromData);
    const second = await loadTabModule("export default function Tab() { return 2; }", fromData);
    expect(first).not.toBe(second);
    expect([
      (first as unknown as () => number)(),
      (second as unknown as () => number)(),
    ]).toEqual([
      1,
      2,
    ]);
  });

  it("refuses a module with no component as its default export", async () => {
    await expect(loadTabModule("export const Tab = () => null;", fromData)).rejects.toThrow(
      "tab.js has no component as its default export",
    );
    await expect(loadTabModule("export default 3;", fromData)).rejects.toThrow(
      "tab.js has no component as its default export",
    );
  });

  it("says a module that did not load did not, and why", async () => {
    await expect(loadTabModule("export default function (", fromData)).rejects.toThrow(
      /^tab\.js did not load: /,
    );
    await expect(
      loadTabModule("throw new Error('boom'); export default () => null;", fromData),
    ).rejects.toThrow("tab.js did not load: boom");
  });
});

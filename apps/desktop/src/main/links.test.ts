// What main hands to the OS when a person clicks a link in a model's text.
// The renderer sends this address, so the scheme check is all that stands
// between a click and an arbitrary URL handler. Electron is mocked, as in `forge.test.ts`.

import { beforeEach, describe, expect, it, vi } from "vitest";

const openExternal = vi.fn(async (_url: string) => undefined);
vi.mock("electron", () => ({ shell: { openExternal: (url: string) => openExternal(url) } }));

const { openLink } = await import("./links");

beforeEach(() => openExternal.mockClear());

describe("opening a link in a model's text", () => {
  it("hands an https address to the OS", async () => {
    await expect(openLink("https://github.com/tokio-rs/tokio/pull/7012")).resolves.toEqual({
      ok: true,
    });
    expect(openExternal).toHaveBeenCalledWith("https://github.com/tokio-rs/tokio/pull/7012");
  });

  it("hands an http address to the OS", async () => {
    await expect(openLink("http://localhost:5173/")).resolves.toEqual({ ok: true });
  });

  it.each(["javascript:alert(1)", "file:///Users/user/.ssh/id_ed25519", "vscode://file/x", "../x"])(
    "refuses %s by name",
    async (address) => {
      await expect(openLink(address)).resolves.toEqual({
        ok: false,
        why: "not_addressable",
        address,
      });
      expect(openExternal).not.toHaveBeenCalled();
    },
  );

  it("carries the OS's own refusal through", async () => {
    openExternal.mockRejectedValueOnce(new Error("no application knows how to open this"));
    await expect(openLink("https://example.com")).resolves.toMatchObject({
      ok: false,
      why: "refused",
      detail: "Error: no application knows how to open this",
    });
  });
});

// What main will fetch for a forge attachment, and who gets the `gh` token.
//
// **The fetcher and the token are injected**, so no test reaches the network or runs `gh`. What is asserted is
// each address the fetcher was asked and the headers it was asked with, since a token sent to the wrong host
// cannot be taken back.

import { describe, expect, it, vi } from "vitest";

import { fetchAttachment, MOST_BYTES } from "./attachments";
import type { Fetcher } from "./attachments";

const ASSET = "https://github.com/user-attachments/assets/0b7c1f0e-2f1e-4d52-9a6e-000000000001";
const STORAGE = "https://objects.githubusercontent.com/github-production-user-asset/1";
const ELSEWHERE = "https://example.com/video.mp4";

const token = vi.fn(async () => "gho_secret");

/** A fetcher that answers each address from `by`, and keeps what it was asked. */
function serving(by: Record<string, Response>) {
  const asked: { address: string; headers: Record<string, string> }[] = [];
  const fetcher: Fetcher = async (address, init) => {
    asked.push({ address, headers: init.headers });
    const response = by[address];
    if (response === undefined) throw new Error(`unexpected fetch of ${address}`);
    return response;
  };
  return { fetcher, asked };
}

const redirect = (to: string) => new Response(null, { status: 302, headers: { location: to } });
const video = (extra: Record<string, string> = {}) =>
  new Response(new Uint8Array([1, 2, 3]), { status: 200, headers: { "content-type": "video/mp4; codecs=avc1", ...extra } });

describe("fetching a forge attachment", () => {
  it("returns the bytes with the mime from content-type", async () => {
    const { fetcher } = serving({ [ASSET]: video() });
    const got = await fetchAttachment(ASSET, fetcher, token);
    expect(got).toEqual({ bytes: new Uint8Array([1, 2, 3]), mime: "video/mp4" });
  });

  it("refuses a host that is not GitHub's, without asking it", async () => {
    const { fetcher, asked } = serving({ [ELSEWHERE]: video() });
    for (const address of [ELSEWHERE, "https://github.com.evil.test/a.mp4", "http://github.com/a.mp4", "not a url", 7]) {
      expect(await fetchAttachment(address, fetcher, token)).toBeNull();
    }
    expect(asked).toEqual([]);
  });

  it("sends the token to github.com", async () => {
    const { fetcher, asked } = serving({ [ASSET]: video() });
    await fetchAttachment(ASSET, fetcher, token);
    expect(asked).toEqual([{ address: ASSET, headers: { authorization: "token gho_secret" } }]);
  });

  it("does not forward the token after a redirect to another host", async () => {
    const { fetcher, asked } = serving({ [ASSET]: redirect(STORAGE), [STORAGE]: video() });
    const got = await fetchAttachment(ASSET, fetcher, token);
    expect(got?.mime).toBe("video/mp4");
    expect(asked).toEqual([
      { address: ASSET, headers: { authorization: "token gho_secret" } },
      { address: STORAGE, headers: {} },
    ]);
  });

  it("refuses a redirect to a host that is not allowed, without asking it", async () => {
    const { fetcher, asked } = serving({ [ASSET]: redirect(ELSEWHERE), [ELSEWHERE]: video() });
    expect(await fetchAttachment(ASSET, fetcher, token)).toBeNull();
    expect(asked.map((one) => one.address)).toEqual([ASSET]);
  });

  it("stops after five redirect hops", async () => {
    const loop = serving({ [ASSET]: redirect(STORAGE), [STORAGE]: redirect(STORAGE) });
    expect(await fetchAttachment(ASSET, loop.fetcher, token)).toBeNull();
    expect(loop.asked).toHaveLength(6);
  });

  it("refuses a body over the size cap, declared or not", async () => {
    const declared = serving({ [ASSET]: video({ "content-length": String(MOST_BYTES + 1) }) });
    expect(await fetchAttachment(ASSET, declared.fetcher, token)).toBeNull();
    const undeclared = serving({ [ASSET]: new Response(new Uint8Array(MOST_BYTES + 1), { status: 200 }) });
    expect(await fetchAttachment(ASSET, undeclared.fetcher, token)).toBeNull();
  });
});

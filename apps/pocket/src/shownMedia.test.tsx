// @vitest-environment happy-dom
// What a Job showed, read on the phone: a video plays from an address it asks the Mac for, and a picture's blob is given back
// when its screen goes.

import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";

import { call, getJson, Refused } from "./client";
import { readShownMedia } from "./jobState";
import type { Machine } from "./machines";
import { FileView } from "./Shown";

const machine = {
  origin: "https://studio.tail0.ts.net",
  name: "Studio",
} as Machine;

vi.mock("./client", async (real) => ({
  ...(await real<typeof import("./client")>()),
  call: vi.fn(),
  getJson: vi.fn(),
}));
vi.mock("./fleet", async (real) => ({
  ...(await real<typeof import("./fleet")>()),
  macOf: () => ({
    machine,
  }),
  start: async () => {},
  useFleet: () => ({}),
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const said = vi.mocked(getJson);
const called = vi.mocked(call);
const VIDEO = "/work/run with space.mp4";
const ASKED = `/api/sessions/shown/address?session_id=S1&path=${encodeURIComponent(VIDEO)}`;
const addressed = (address: string) => ({
  address,
  media_type: "video/mp4",
});

afterEach(() => {
  vi.restoreAllMocks();
  said.mockReset();
  called.mockReset();
});

describe("readShownMedia", () => {
  it("a video asks the Mac for its address and plays from the Mac's origin", async () => {
    said.mockResolvedValueOnce(addressed("/media/one?sig=a"));
    const media = await readShownMedia(machine, "S1", VIDEO);
    expect(said).toHaveBeenCalledExactlyOnceWith(machine, ASKED);
    expect(media).toMatchObject({
      kind: "video",
      url: `${machine.origin}/media/one?sig=a`,
    });
    expect(called).not.toHaveBeenCalled();
  });

  it("`again` asks afresh, and answers with the new address", async () => {
    said.mockResolvedValueOnce(addressed("/media/one?sig=a"));
    said.mockResolvedValueOnce(addressed("/media/one?sig=b"));
    const media = await readShownMedia(machine, "S1", VIDEO);
    if (media.kind !== "video") throw new Error(media.kind);
    await expect(media.again?.()).resolves.toBe(`${machine.origin}/media/one?sig=b`);
    expect(said).toHaveBeenCalledTimes(2);
    expect(said).toHaveBeenLastCalledWith(machine, ASKED);
  });

  it("`again` refused rejects, for the row to draw its line", async () => {
    said.mockResolvedValueOnce(addressed("/media/one?sig=a"));
    said.mockRejectedValueOnce(new Refused(404, "That Job did not show that file."));
    const media = await readShownMedia(machine, "S1", VIDEO);
    if (media.kind !== "video") throw new Error(media.kind);
    await expect(media.again?.()).rejects.toThrow("That Job did not show that file.");
  });

  it("`open` goes to the address asked for last, since an older one may have lapsed", async () => {
    const opened = vi.spyOn(window, "open").mockReturnValue(null);
    said.mockResolvedValueOnce(addressed("/media/one?sig=a"));
    said.mockResolvedValueOnce(addressed("/media/one?sig=b"));
    const media = await readShownMedia(machine, "S1", VIDEO);
    if (media.kind !== "video") throw new Error(media.kind);
    media.open?.();
    expect(opened).toHaveBeenLastCalledWith(
      `${machine.origin}/media/one?sig=a`,
      "_blank",
      "noopener",
    );
    await media.again?.();
    media.open?.();
    expect(opened).toHaveBeenLastCalledWith(
      `${machine.origin}/media/one?sig=b`,
      "_blank",
      "noopener",
    );
  });

  it("a refused address draws the Mac's words, and offers nothing to open", async () => {
    said.mockRejectedValueOnce(new Refused(404, "That Job did not show that file."));
    await expect(readShownMedia(machine, "S1", VIDEO)).resolves.toEqual({
      kind: "refused",
      says: "That Job did not show that file.",
    });
  });

  it("a picture is its bytes as a blob, and never asks for an address", async () => {
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:one");
    called.mockResolvedValueOnce(
      new Response(new Uint8Array(4), {
        headers: {
          "Content-Type": "image/png",
        },
      }),
    );
    await expect(readShownMedia(machine, "S1", "/work/a.png")).resolves.toEqual({
      kind: "picture",
      url: "blob:one",
    });
    expect(said).not.toHaveBeenCalled();
  });

  it("a picture the Mac refuses keeps the Mac's words", async () => {
    called.mockRejectedValueOnce(new Refused(413, "This picture is larger than 8 MB."));
    await expect(readShownMedia(machine, "S1", "/work/a.png")).resolves.toEqual({
      kind: "refused",
      says: "This picture is larger than 8 MB.",
    });
  });

  it("a picture's path that answers with text is refused, and draws no text as a picture", async () => {
    called.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          path: "/work/a.png",
          text: "not a picture",
        }),
        {
          headers: {
            "Content-Type": "application/json",
          },
        },
      ),
    );
    await expect(readShownMedia(machine, "S1", "/work/a.png")).resolves.toEqual({
      kind: "refused",
      says: "This picture cannot be drawn here.",
    });
  });
});

describe("FileView", () => {
  let host: HTMLElement;
  let revoked: MockInstance<(url: string) => void>;

  beforeEach(() => {
    host = document.createElement("div");
    document.body.append(host);
    vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:shown");
    revoked = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
  });
  afterEach(() => host.remove());

  const picture = () =>
    new Response(new Uint8Array(4), {
      headers: {
        "Content-Type": "image/png",
      },
    });
  const mount = () => {
    const root = createRoot(host);
    act(() => root.render(<FileView origin={machine.origin} id="S1" path="/work/a.png" />));
    return root;
  };
  const settled = () => act(() => new Promise<void>((done) => setTimeout(done, 20)));

  it("gives a drawn picture's blob back when the screen goes, and not before", async () => {
    called.mockResolvedValueOnce(picture());
    const root = mount();
    await settled();
    expect(host.querySelector("img")?.getAttribute("src")).toBe("blob:shown");
    expect(revoked).not.toHaveBeenCalled();
    act(() => root.unmount());
    expect(revoked).toHaveBeenCalledExactlyOnceWith("blob:shown");
  });

  it("gives back a blob that arrives after the screen has gone", async () => {
    let answer = (_: Response) => {};
    called.mockReturnValueOnce(
      new Promise<Response>((done) => {
        answer = done;
      }),
    );
    const root = mount();
    act(() => root.unmount());
    expect(revoked).not.toHaveBeenCalled();
    answer(picture());
    await settled();
    expect(revoked).toHaveBeenCalledExactlyOnceWith("blob:shown");
  });

  it("has no blob to give back for a file that reads as text", async () => {
    called.mockResolvedValueOnce(
      new Response(
        JSON.stringify({
          path: "/work/a.png",
          text: "words",
        }),
        {
          headers: {
            "Content-Type": "application/json",
          },
        },
      ),
    );
    const root = mount();
    await settled();
    act(() => root.unmount());
    expect(revoked).not.toHaveBeenCalled();
  });
});

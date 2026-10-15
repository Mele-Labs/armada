// A shared sketch is written where the person says and nowhere else, in a format the dialog offered.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ app: { getPath: () => "/Downloads" }, BrowserWindow: { fromWebContents: () => null }, dialog: {} }));

const { saveSketchFile, MOST_BYTES } = await import("./sketch-share");

const bytes = (text: string) => new TextEncoder().encode(text);

function host(chosen: string | undefined) {
  const write = vi.fn(async (_path: string, _bytes: Uint8Array) => undefined);
  const choose = vi.fn(async (_options: { defaultName: string; filters: { name: string; extensions: string[] }[] }) => chosen);
  return { write, choose };
}

describe("saving a sketch", () => {
  it("offers each format as a file type, the first as the name, and writes where the person chose", async () => {
    const h = host("/Users/me/Desktop/How data moves.png");
    const saved = await saveSketchFile(
      [
        { kind: "png", name: "How data moves.png", bytes: bytes("png") },
        { kind: "svg", name: "How data moves.svg", bytes: bytes("svg") },
      ],
      h,
    );
    expect(saved).toEqual({ saved: true, name: "How data moves.png" });
    expect(h.choose).toHaveBeenCalledWith({
      defaultName: "How data moves.png",
      filters: [{ name: "PNG image", extensions: ["png"] }, { name: "SVG image", extensions: ["svg"] }],
    });
    expect(h.write).toHaveBeenCalledWith("/Users/me/Desktop/How data moves.png", bytes("png"));
  });

  it("writes the format whose extension they chose", async () => {
    const h = host("/tmp/x.svg");
    await saveSketchFile([{ kind: "png", name: "a.png", bytes: bytes("png") }, { kind: "svg", name: "a.svg", bytes: bytes("svg") }], h);
    expect(h.write).toHaveBeenCalledWith("/tmp/x.svg", bytes("svg"));
  });

  it("puts the first format's extension on a name they typed over with another, so a page never lands as `.sh`", async () => {
    const h = host("/tmp/run.sh");
    expect(await saveSketchFile([{ kind: "html", name: "a.html", bytes: bytes("<p>") }], h)).toEqual({ saved: true, name: "run.sh.html" });
    expect(h.write).toHaveBeenCalledWith("/tmp/run.sh.html", bytes("<p>"));
  });

  it("writes nothing and fails nothing where the dialog is closed", async () => {
    const h = host(undefined);
    expect(await saveSketchFile([{ kind: "html", name: "a.html", bytes: bytes("<p>") }], h)).toEqual({ saved: false, why: "cancelled" });
    expect(h.write).not.toHaveBeenCalled();
  });

  it("says so when the disk refuses", async () => {
    const h = host("/tmp/a.html");
    h.write.mockRejectedValueOnce(new Error("EACCES"));
    expect(await saveSketchFile([{ kind: "html", name: "a.html", bytes: bytes("<p>") }], h)).toEqual({ saved: false, why: "failed" });
  });

  it("takes only a name, never a place: folders in it are dropped and the extension is the kind's", async () => {
    const h = host("/tmp/ok.html");
    await saveSketchFile([{ kind: "html", name: "../../etc/passwd", bytes: bytes("<p>") }], h);
    expect(h.choose.mock.calls[0]![0].defaultName).toBe("passwd.html");
  });

  it("opens no dialog for a request it does not take", async () => {
    const h = host("/tmp/a");
    const refused = { saved: false, why: "failed" };
    expect(await saveSketchFile("a", h)).toEqual(refused);
    expect(await saveSketchFile([], h)).toEqual(refused);
    expect(await saveSketchFile([{ kind: "exe", name: "a.exe", bytes: bytes("x") }], h)).toEqual(refused);
    expect(await saveSketchFile([{ kind: "html", name: "a.html", bytes: "<p>" }], h)).toEqual(refused);
    expect(await saveSketchFile([{ kind: "html", name: "a.html", bytes: new Uint8Array(0) }], h)).toEqual(refused);
    expect(await saveSketchFile([{ kind: "html", name: "a.html", bytes: new Uint8Array(MOST_BYTES + 1) }], h)).toEqual(refused);
    expect(await saveSketchFile([{ kind: "html", name: "a", bytes: bytes("x") }, { kind: "html", name: "b", bytes: bytes("x") }], h)).toEqual(refused);
    expect(h.choose).not.toHaveBeenCalled();
  });
});

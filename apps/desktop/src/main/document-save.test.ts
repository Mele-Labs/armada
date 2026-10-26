// A Job's document is written where the person says and nowhere else, as the page it is or printed from that page.

import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ BrowserWindow: {}, session: {} }));

const { saveDocumentFile, MOST_CHARS } = await import("./document-save");

const PAGE = "<!doctype html><html><body><p>Findings</p></body></html>";

function host(chosen: string | undefined) {
  const write = vi.fn(async (_path: string, _bytes: Uint8Array) => undefined);
  const choose = vi.fn(async (_options: { defaultName: string; filters: { name: string; extensions: string[] }[] }) => chosen);
  const print = vi.fn(async (_html: string) => new TextEncoder().encode("%PDF"));
  return { write, choose, print };
}

describe("saving a document", () => {
  it("offers the title as the name, and writes the page as it is where the person chose", async () => {
    const h = host("/Users/me/Desktop/Findings.html");
    expect(await saveDocumentFile({ kind: "html", name: "Findings", html: PAGE }, h)).toEqual({ saved: true, name: "Findings.html" });
    expect(h.choose).toHaveBeenCalledWith({ defaultName: "Findings.html", filters: [{ name: "Web page", extensions: ["html"] }] });
    expect(h.write).toHaveBeenCalledWith("/Users/me/Desktop/Findings.html", new TextEncoder().encode(PAGE));
    expect(h.print).not.toHaveBeenCalled();
  });

  it("prints the same page for a PDF, and writes what was printed", async () => {
    const h = host("/tmp/Findings.pdf");
    expect(await saveDocumentFile({ kind: "pdf", name: "Findings.pdf", html: PAGE }, h)).toEqual({ saved: true, name: "Findings.pdf" });
    expect(h.choose).toHaveBeenCalledWith({ defaultName: "Findings.pdf", filters: [{ name: "PDF", extensions: ["pdf"] }] });
    expect(h.print).toHaveBeenCalledWith(PAGE);
    expect(h.write).toHaveBeenCalledWith("/tmp/Findings.pdf", new TextEncoder().encode("%PDF"));
  });

  it("keeps a folder out of the name and puts the extension back on a place typed without it", async () => {
    const h = host("/tmp/out");
    await saveDocumentFile({ kind: "html", name: "../../etc/Findings: q?", html: PAGE }, h);
    expect(h.choose.mock.calls[0]![0].defaultName).toBe("Findings  q.html");
    expect(h.write.mock.calls[0]![0]).toBe("/tmp/out.html");
  });

  it("writes nothing where the dialog is closed, and says it was not a failure", async () => {
    const h = host(undefined);
    expect(await saveDocumentFile({ kind: "pdf", name: "Findings", html: PAGE }, h)).toEqual({ saved: false, why: "cancelled" });
    expect(h.print).not.toHaveBeenCalled();
    expect(h.write).not.toHaveBeenCalled();
  });

  it("says it failed when printing or writing does", async () => {
    const printing = host("/tmp/a.pdf");
    printing.print.mockRejectedValueOnce(new Error("no window"));
    expect(await saveDocumentFile({ kind: "pdf", name: "a", html: PAGE }, printing)).toEqual({ saved: false, why: "failed" });
    const writing = host("/tmp/a.html");
    writing.write.mockRejectedValueOnce(new Error("disk full"));
    expect(await saveDocumentFile({ kind: "html", name: "a", html: PAGE }, writing)).toEqual({ saved: false, why: "failed" });
  });

  it("refuses a request that is not a kind, a name and a page, without asking where", async () => {
    const h = host("/tmp/a.html");
    for (const request of [undefined, null, "x", { kind: "docx", name: "a", html: PAGE }, { kind: "html", name: 1, html: PAGE }, { kind: "html", name: "a", html: "" }, { kind: "html", name: "a", html: "x".repeat(MOST_CHARS + 1) }]) {
      expect(await saveDocumentFile(request, h)).toEqual({ saved: false, why: "failed" });
    }
    expect(h.choose).not.toHaveBeenCalled();
  });
});

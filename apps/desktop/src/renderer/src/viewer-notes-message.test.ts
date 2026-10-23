// Notes on a sketch, a file or a picture as the one message a Job is sent: what each kind says about its place, the
// numbering across kinds, the pictures that go along, and the thread reading the same words back into notes.

import { describe, expect, it } from "vitest";
import { notedOf, notesMessage } from "@armada/components";
import type { HeldNote } from "@armada/components";

const PNG = "data:image/png;base64,iVBORw0KGgo=";

const sketch = (text: string): HeldNote => ({
  about: {
    kind: "sketch",
    title: "Retry flow",
    parts: [
      { id: "n3", label: "Backoff" },
      { id: "n4", label: "Cap" },
    ],
  },
  text,
});
const file = (text: string, quote = "let a = 1;", from = 12, to = 12): HeldNote => ({
  about: { kind: "file", path: "crates/fleet/src/retry.rs", from, to, quote },
  text,
});
const picture = (text: string, crop = PNG, region?: { x: number; y: number; w: number; h: number }): HeldNote => ({
  about: { kind: "image", name: "shot.png", path: "/tmp/shot.png", crop, ...(region === undefined ? {} : { region }) },
  text,
});

describe("a sketch note", () => {
  it("names the sketch by its title and the parts by their ids and labels", () => {
    expect(notesMessage([sketch("The cap is too low")]).text).toBe('1. Sketch "Retry flow", parts n3 (Backoff), n4 (Cap)\n   The cap is too low');
  });

  it("says part, not parts, for one", () => {
    const one: HeldNote = { about: { kind: "sketch", title: "Retry flow", parts: [{ id: "n3", label: "Backoff" }] }, text: "Why" };
    expect(notesMessage([one]).text).toBe('1. Sketch "Retry flow", part n3 (Backoff)\n   Why');
  });

  it("sends no picture", () => {
    expect(notesMessage([sketch("x")]).pictures).toEqual([]);
  });
});

describe("a file note", () => {
  it("names the path and the lines, quotes them, then says the note", () => {
    const message = notesMessage([file("Too many", "let a = 1;\nlet b = 2;\nlet c = 3;\n", 12, 14)]);
    expect(message.text).toBe("1. File crates/fleet/src/retry.rs:12-14\n   > let a = 1;\n   > let b = 2;\n   > let c = 3;\n   Too many");
  });

  it("names one line as that line", () => {
    expect(notesMessage([file("Why")]).text.split("\n")[0]).toBe("1. File crates/fleet/src/retry.rs:12");
  });

  it("quotes six lines whole", () => {
    const six = ["a", "b", "c", "d", "e", "f"].join("\n");
    expect(notesMessage([file("Why", six, 1, 6)]).text).toBe("1. File crates/fleet/src/retry.rs:1-6\n   > a\n   > b\n   > c\n   > d\n   > e\n   > f\n   Why");
  });

  it("quotes the first six lines of more, then marks the rest with an ellipsis", () => {
    const eight = ["a", "b", "c", "d", "e", "f", "g", "h"].join("\n");
    expect(notesMessage([file("Why", eight, 1, 8)]).text).toBe("1. File crates/fleet/src/retry.rs:1-8\n   > a\n   > b\n   > c\n   > d\n   > e\n   > f\n   > …\n   Why");
  });

  it("quotes a blank line without trailing space", () => {
    expect(notesMessage([file("Why", "a\n\nb", 1, 3)]).text).toBe("1. File crates/fleet/src/retry.rs:1-3\n   > a\n   >\n   > b\n   Why");
  });
});

describe("a picture note", () => {
  it("names where it is read from, the region in percent and the file it is attached as", () => {
    const message = notesMessage([picture("Misaligned", PNG, { x: 0.1, y: 0.2, w: 0.5, h: 0.25 })]);
    expect(message.text).toBe("1. Picture /tmp/shot.png, region x 10-60% y 20-45%, attached as picture-1.png\n   Misaligned");
    expect(message.pictures).toEqual([{ name: "picture-1.png", png: PNG }]);
  });

  it("names the whole picture without a region", () => {
    expect(notesMessage([picture("Whole")]).text.split("\n")[0]).toBe("1. Picture /tmp/shot.png, attached as picture-1.png");
  });

  it("falls back to the picture's name where it has no path", () => {
    const note: HeldNote = { about: { kind: "image", name: "shot.png", crop: PNG }, text: "x" };
    expect(notesMessage([note]).text.split("\n")[0]).toBe("1. Picture shot.png, attached as picture-1.png");
  });

  it("attaches nothing, and says nothing of an attachment, where no crop could be read", () => {
    const message = notesMessage([picture("Cross-origin", "")]);
    expect(message.text).toBe("1. Picture /tmp/shot.png\n   Cross-origin");
    expect(message.pictures).toEqual([]);
  });
});

describe("notes together", () => {
  const mixed = [file("one"), picture("two", PNG), sketch("three"), picture("four", "data:image/png;base64,AAAA")];

  it("numbers them in the order held, one block each, with a blank line between", () => {
    const heads = notesMessage(mixed).text.split("\n\n").map((block) => block.split("\n")[0]);
    expect(heads).toEqual([
      "1. File crates/fleet/src/retry.rs:12",
      "2. Picture /tmp/shot.png, attached as picture-1.png",
      '3. Sketch "Retry flow", parts n3 (Backoff), n4 (Cap)',
      "4. Picture /tmp/shot.png, attached as picture-2.png",
    ]);
  });

  it("numbers pictures apart from notes, and each name matches the file that goes along", () => {
    const message = notesMessage(mixed);
    expect(message.pictures).toEqual([
      { name: "picture-1.png", png: PNG },
      { name: "picture-2.png", png: "data:image/png;base64,AAAA" },
    ]);
  });

  it("indents a many-line note under its heading and keeps its blank lines", () => {
    expect(notesMessage([sketch("first\n\nsecond")]).text).toBe('1. Sketch "Retry flow", parts n3 (Backoff), n4 (Cap)\n   first\n\n   second');
  });

  it("indents past a two-digit number", () => {
    const text = notesMessage(Array.from({ length: 10 }, (_, at) => sketch(`note ${String(at + 1)}`))).text;
    expect(text).toContain('10. Sketch "Retry flow", parts n3 (Backoff), n4 (Cap)\n    note 10');
  });
});

describe("a notes message read back", () => {
  it("gives each note its number, what it named and its body", () => {
    const text = notesMessage([
      sketch("The cap is too low"),
      file("Too many", "let a = 1;\nlet b = 2;", 12, 14),
      picture("Misaligned", PNG, { x: 0.1, y: 0.2, w: 0.5, h: 0.25 }),
    ]).text;
    const blocks = notedOf(text)!;
    expect(blocks.map((one) => one.n)).toEqual([1, 2, 3]);
    expect(blocks[0]).toEqual({
      n: 1,
      ref: {
        kind: "sketch",
        title: "Retry flow",
        parts: [
          { id: "n3", label: "Backoff" },
          { id: "n4", label: "Cap" },
        ],
      },
      body: "The cap is too low",
    });
    expect(blocks[1]).toEqual({ n: 2, ref: { kind: "file", path: "crates/fleet/src/retry.rs", from: 12, to: 14 }, body: "> let a = 1;\n> let b = 2;\nToo many" });
    const [, , third] = blocks;
    expect(third!.ref.kind).toBe("image");
    if (third!.ref.kind !== "image") return;
    expect(third!.ref.path).toBe("/tmp/shot.png");
    expect(third!.ref.region!.x).toBeCloseTo(0.1);
    expect(third!.ref.region!.y).toBeCloseTo(0.2);
    expect(third!.ref.region!.w).toBeCloseTo(0.5);
    expect(third!.ref.region!.h).toBeCloseTo(0.25);
    expect(third!.body).toBe("Misaligned");
  });

  it("reads one line of a file as that line", () => {
    expect(notedOf(notesMessage([file("Why")]).text)![0]!.ref).toEqual({ kind: "file", path: "crates/fleet/src/retry.rs", from: 12, to: 12 });
  });

  it("reads a picture with no attachment and no region as its path alone", () => {
    expect(notedOf(notesMessage([picture("x", "")]).text)![0]!.ref).toEqual({ kind: "image", path: "/tmp/shot.png" });
  });

  it("keeps a note's blank lines and reads past a two-digit number", () => {
    const blocks = notedOf(notesMessage(Array.from({ length: 10 }, (_, at) => sketch(at === 9 ? "last\n\nlines" : `note ${String(at + 1)}`))).text)!;
    expect(blocks).toHaveLength(10);
    expect(blocks[9]).toMatchObject({ n: 10, body: "last\n\nlines" });
  });

  it("is not a notes message when it is any other text", () => {
    expect(notedOf("Please look at the retry code")).toBeUndefined();
    expect(notedOf("")).toBeUndefined();
  });

  it("is not a notes message when one line of it is not", () => {
    const text = notesMessage([sketch("a"), file("b")]).text;
    expect(notedOf(`${text}\nand also this`)).toBeUndefined();
  });

  it("is not a notes message when the numbers do not run from 1", () => {
    expect(notedOf(notesMessage([sketch("a"), file("b")]).text.replace("2. File", "3. File"))).toBeUndefined();
  });
});

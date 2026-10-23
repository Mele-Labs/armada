/** What a note points at, as each viewer reports it. Lines are 1-based and inclusive. A crop is a PNG `data:` address. */
export type NoteAbout =
  | { kind: "sketch"; title: string; parts: readonly { id: string; label: string }[] }
  | { kind: "file"; path: string; from: number; to: number; quote: string }
  | { kind: "image"; name: string; region?: { x: number; y: number; w: number; h: number }; crop: string };

export type HeldNote = { about: NoteAbout; text: string };

/** A picture that goes with the message, named as the text names it. */
export type NotePicture = { name: string; png: string };

export type NotesMessage = { text: string; pictures: NotePicture[] };

/** The most lines of a file a note quotes. */
const QUOTED = 6;

const lines = (from: number, to: number) => (from === to ? String(from) : `${String(from)}-${String(to)}`);

function heading(about: NoteAbout, picture: number | undefined): string {
  if (about.kind === "sketch") return `${about.title}: ${about.parts.map((one) => one.label).join(", ")}`;
  if (about.kind === "file") return `${about.path}:${lines(about.from, about.to)}`;
  return `${about.name}${about.region === undefined ? "" : ", region"} (picture ${String(picture)})`;
}

function quoted(about: NoteAbout): string[] {
  if (about.kind !== "file") return [];
  const all = about.quote.replace(/\n+$/, "").split("\n");
  const shown = all.slice(0, QUOTED).map((line) => `> ${line}`.trimEnd());
  return all.length > QUOTED ? [...shown, "> …"] : shown;
}

/**
 * Held notes as one message: a numbered block per note, its subject on the number's line, a file's lines quoted under it,
 * then the note. A picture's note names the picture it travels as, `(picture N)`, in the order `pictures` holds them.
 */
export function notesMessage(notes: readonly HeldNote[]): NotesMessage {
  const pictures: NotePicture[] = [];
  const blocks = notes.map((note, at) => {
    const n = at + 1;
    let picture: number | undefined;
    if (note.about.kind === "image") {
      picture = pictures.length + 1;
      pictures.push({ name: `picture-${String(picture)}.png`, png: note.about.crop });
    }
    const indent = " ".repeat(String(n).length + 2);
    const body = [...quoted(note.about), ...note.text.trim().split("\n")].map((line) => (line === "" ? "" : indent + line));
    return [`${String(n)}. ${heading(note.about, picture)}`, ...body].join("\n");
  });
  return { text: blocks.join("\n\n"), pictures };
}

/** What a note is about, short, for the note box and a mark's tooltip. */
export function aboutSaid(about: NoteAbout): string {
  if (about.kind === "sketch") return about.parts.map((one) => one.label).join(", ");
  if (about.kind === "file") return `${about.path.split("/").pop() ?? about.path}:${lines(about.from, about.to)}`;
  return about.region === undefined ? about.name : `${about.name}, region`;
}

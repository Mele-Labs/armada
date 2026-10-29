// A picture of a whole Bridge window on the backoff table, as a note's capture is once it is the window rather than the
// element: drawn as an SVG at a window's size, so the panel shrinks it as it would a real one. `WINDOW_BOXES` are the
// boxes a note can ring on it, in its pixels. Draft data for `scenarios/walk-held-notes.ts`.

const WIDTH = 1440;
const HEIGHT = 900;

// The dark theme's own values, `packages/tokens/tokens.css`: the picture is of Bridge, so it is drawn in Bridge's colours.
const BASE = "#0F1419";
const RAISED = "#161C23";
const RULE = "#232B35";
const EDGE = "#2E3946";
const INK = "#E4E9EF";
const MUTED = "#93A1B1";
const SUBTLE = "#7E8CA0";
const SELECTED = "#1C3A52";
const RUNNING = "#4FB8D9";
const PASSED = "#4FAF7C";
const YOU = "#E8C547";

const words = (x: number, y: number, text: string, fill = INK, size = 14, weight = 400) =>
  `<text x="${x}" y="${y}" fill="${fill}" font-size="${size}" font-weight="${weight}">${text}</text>`;
const box = (x: number, y: number, width: number, height: number, fill: string, radius = 0, stroke?: string, opacity?: number) =>
  `<rect x="${x}" y="${y}" width="${width}" height="${height}" rx="${radius}" fill="${fill}"${stroke === undefined ? "" : ` stroke="${stroke}"`}${opacity === undefined ? "" : ` fill-opacity="${opacity}"`}/>`;
/** A line of prose, drawn as a bar: the picture is too small for its words to be read. */
const line = (x: number, y: number, width: number) => box(x, y, width, 8, EDGE, 4);

const PLACES = ["Cockpit", "Jobs", "Sessions", "Studios", "Logbook", "System"];
const JOBS: [string, string][] = [
  ["Retry backoff cap", RUNNING],
  ["Pin the store clock", PASSED],
  ["Slot pool size", PASSED],
  ["Doctor palette", SUBTLE],
  ["Lessons page copy", SUBTLE],
];
const ROWS: [string, string, string][] = [
  ["1", "1 s", "30 s"],
  ["2", "2 s", "30 s"],
  ["3", "4 s", "30 s"],
  ["4", "8 s", "30 s"],
  ["5", "16 s", "30 s"],
  ["6", "30", "30"],
];

const TABLE = { x: 352, y: 132, width: 656, head: 40, row: 44 };

/** Where each note can be, in the picture's pixels. */
export const WINDOW_BOXES = {
  heading: { x: 348, y: 78, width: 168, height: 30 },
  capCell: { x: 784, y: TABLE.y + TABLE.head + 4 * TABLE.row, width: 224, height: TABLE.row },
  lastRow: { x: TABLE.x, y: TABLE.y + TABLE.head + 5 * TABLE.row, width: TABLE.width, height: TABLE.row },
  jobRow: { x: 8, y: 96, width: 304, height: 52 },
} as const;

function svg(): string {
  const parts: string[] = [box(0, 0, WIDTH, HEIGHT, BASE)];
  // The title bar: the window's buttons and the places.
  parts.push(box(0, 0, WIDTH, 44, RAISED), box(0, 44, WIDTH, 1, RULE));
  parts.push(`<circle cx="20" cy="22" r="6" fill="#FF5F57"/><circle cx="40" cy="22" r="6" fill="#FEBC2E"/><circle cx="60" cy="22" r="6" fill="#28C840"/>`);
  PLACES.forEach((name, at) => parts.push(words(120 + at * 92, 27, name, at === 1 ? INK : MUTED, 13, at === 1 ? 500 : 400)));
  // The Jobs list.
  parts.push(box(0, 45, 320, HEIGHT - 45, RAISED), box(320, 45, 1, HEIGHT - 45, RULE), words(20, 78, "Jobs", MUTED, 12, 500));
  JOBS.forEach(([title, state], at) => {
    const y = 96 + at * 56;
    if (at === 0) parts.push(box(8, y, 304, 52, SELECTED, 6));
    parts.push(`<circle cx="28" cy="${y + 20}" r="4" fill="${state}"/>`, words(42, y + 25, title, INK, 13, 500), line(42, y + 36, 120 + ((at * 37) % 80)));
  });
  // The page: its heading, the table and the prose under it.
  parts.push(words(352, 101, "Backoff table", INK, 22, 500), line(352, 116, 280));
  parts.push(box(TABLE.x, TABLE.y, TABLE.width, TABLE.head + ROWS.length * TABLE.row, RAISED, 6, EDGE));
  const columns = [368, 560, 800];
  ["Attempt", "Delay", "Cap"].forEach((name, at) => parts.push(words(columns[at]!, TABLE.y + 25, name, MUTED, 12, 500)));
  ROWS.forEach((cells, at) => {
    const y = TABLE.y + TABLE.head + at * TABLE.row;
    parts.push(box(TABLE.x, y, TABLE.width, 1, RULE));
    cells.forEach((cell, column) => parts.push(words(columns[column]!, y + 27, cell, INK, 14)));
  });
  [480, 560, 420, 520].forEach((width, at) => parts.push(line(352, 478 + at * 20, width)));
  parts.push(box(352, 572, 656, 220, RAISED, 6, EDGE));
  [600, 520, 580, 300, 540, 460].forEach((width, at) => parts.push(line(376, 600 + at * 28, width)));
  // The Job's panel: its title and thread.
  parts.push(box(1040, 45, 400, HEIGHT - 45, RAISED), box(1040, 45, 1, HEIGHT - 45, RULE), words(1064, 80, "Retry backoff cap", INK, 15, 500));
  parts.push(box(1064, 104, 352, 72, BASE, 6, EDGE), box(1064, 104, 352, 22, YOU, 6, undefined, 0.14), words(1076, 120, "You", YOU, 12, 500), line(1076, 144, 280), line(1076, 160, 200));
  parts.push(box(1064, 192, 352, 120, BASE, 6, EDGE), words(1076, 214, "Agent", MUTED, 12, 500));
  [300, 320, 260, 180].forEach((width, at) => parts.push(line(1076, 232 + at * 18, width)));
  parts.push(box(1064, HEIGHT - 96, 352, 72, BASE, 6, EDGE), line(1080, HEIGHT - 72, 160));
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${WIDTH}" height="${HEIGHT}" viewBox="0 0 ${WIDTH} ${HEIGHT}" font-family="-apple-system, system-ui, sans-serif">${parts.join("")}</svg>`;
}

/** The picture as a note's crop carries it: its type and its bytes in base64. */
export function windowPicture(): { media_type: string; data: string } {
  return { media_type: "image/svg+xml", data: btoa(svg()) };
}

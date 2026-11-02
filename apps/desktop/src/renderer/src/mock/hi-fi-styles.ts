// What the mock's host reads for a `mock` node of a sketch: the repository's real stylesheets, by the paths the node names,
// and the IBM Plex files they name, as bytes. Fleet does this in the app, from the Job's checkout; the mock reads this
// repository's own files with Vite so the picture is drawn with what the app draws with. Mock only.

import type { MockFont, MockStyles, ResolveMockStyles } from "@armada/components";

import { BASE, JOB_CARD, TOKENS } from "./sessions/hi-fi";

// Typed by hand, as `scenario.ts` does: Vite's client types are not loaded here.
type Globbing = ImportMeta & {
  glob(pattern: string[], options: { eager: true; query: string; import: "default" }): Record<string, string>;
};

const FILES = (import.meta as Globbing).glob(
  ["../../../../../../packages/tokens/tokens.css", "../../../../../../packages/tokens/src/base.css", "../../../../../../packages/components/src/compositions/JobCard/JobCard.css"],
  { eager: true, query: "?raw", import: "default" },
);
const FONTS = (import.meta as Globbing).glob(["../../../../../../packages/tokens/fonts/*.woff2"], { eager: true, query: "?url", import: "default" });
const file = (name: string) => Object.entries(FILES).find(([path]) => path.endsWith(name))?.[1] ?? "";
const font = (name: string) => Object.entries(FONTS).find(([path]) => path.endsWith(name))?.[1] ?? "";

const tokens = file("tokens/tokens.css");
const base = file("tokens/src/base.css");
const jobCard = file("JobCard/JobCard.css");

type Face = { family: string; weight: string; url: string };

const PLEX: readonly Face[] = [
  { family: "IBM Plex Sans", weight: "400", url: font("IBMPlexSans-400.woff2") },
  { family: "IBM Plex Sans", weight: "500", url: font("IBMPlexSans-500.woff2") },
  { family: "IBM Plex Sans", weight: "600", url: font("IBMPlexSans-600.woff2") },
  { family: "IBM Plex Mono", weight: "400", url: font("IBMPlexMono-400.woff2") },
  { family: "IBM Plex Mono", weight: "500", url: font("IBMPlexMono-500.woff2") },
];

/** A sheet with its addresses taken out, as Fleet does: a font arrives as bytes, never as a `url()` the frame would fetch. */
const withoutFaces = (css: string) => css.replace(/@font-face\s*\{[^}]*\}/g, "");

const KEPT: ReadonlyMap<string, { css: string; faces: readonly Face[] }> = new Map([
  [TOKENS, { css: withoutFaces(tokens), faces: PLEX }],
  [BASE, { css: base, faces: [] }],
  [JOB_CARD, { css: jobCard, faces: [] }],
]);

const held = new Map<string, Promise<MockFont>>();
const bytesOf = (face: Face): Promise<MockFont> => {
  const had = held.get(face.url);
  if (had !== undefined) return had;
  const read = fetch(face.url)
    .then((answer) => answer.arrayBuffer())
    .then((bytes) => ({ family: face.family, weight: face.weight, bytes }));
  held.set(face.url, read);
  return read;
};

export const hiFiStyles: ResolveMockStyles = async ({ styles }): Promise<MockStyles> => {
  const named = styles.filter((path) => KEPT.has(path));
  const faces = [...new Set(named.flatMap((path) => KEPT.get(path)!.faces))];
  const theme = document.documentElement.getAttribute("data-theme");
  return {
    css: named.map((path) => KEPT.get(path)!.css),
    fonts: await Promise.all(faces.map(bytesOf)),
    drawnWith: named,
    ...(theme === null ? {} : { root: { "data-theme": theme } }),
  };
};

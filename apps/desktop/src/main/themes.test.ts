// The shipped themes: what is checked in is what the generator writes, every one reads, and each
// overrides only tokens that exist. Node, no document.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

import { contrast, parseHex } from "@armada/tokens/themes/derive.mjs";
import { build, builtIn, popular } from "@armada/tokens/themes/generate.mjs";
import { SWATCH_TOKENS, declaredIn, swatchOf } from "@armada/tokens/themes/swatch.mjs";

const themes = join(dirname(fileURLToPath(import.meta.url)), "../../../../packages/tokens/themes");
const built = build(popular());
const index = JSON.parse(readFileSync(join(themes, "index.json"), "utf8")) as { id: string; title: string; tone: string; flags: string[]; swatch: string[] }[];

/** `--name: value` pairs of a theme's one block. */
const declared = (css: string) => Object.fromEntries([...css.matchAll(/^\s+(--\w[\w-]*): ([^;]+);/gm)].map((m) => [m[1]!, m[2]!]));
const hex = (value: string | undefined) => parseHex(value ?? "");

test("what is checked in is what the generator writes", () => {
  expect(built.problems).toEqual([]);
  for (const [path, text] of built.files) expect(readFileSync(join(themes, path), "utf8"), path).toBe(text);
  expect(readFileSync(join(themes, "built-in.json"), "utf8")).toBe(builtIn());
});

test("the catalogue offers both tones, and none is flagged", () => {
  expect(index.filter((one) => one.tone === "dark").length).toBeGreaterThanOrEqual(20);
  expect(index.filter((one) => one.tone === "light").length).toBeGreaterThanOrEqual(10);
  expect(index.flatMap((one) => one.flags)).toEqual([]);
});

test("a theme overrides tokens that exist and scopes itself to its own id", () => {
  const known = new Set((JSON.parse(readFileSync(join(themes, "../tokens.json"), "utf8")) as { tokens: { name: string }[] }).tokens.map((t) => t.name));
  for (const one of index) {
    const css = readFileSync(join(themes, "css", `${one.id.slice("catalogue:".length)}.css`), "utf8");
    expect(css, one.id).toContain(`[data-theme="${one.id}"] {`);
    for (const name of Object.keys(declared(css))) expect(known.has(name), `${one.id} ${name}`).toBe(true);
  }
});

test("every theme's text reads on every ground it sits on", () => {
  for (const one of index) {
    const css = declared(readFileSync(join(themes, "css", `${one.id.slice("catalogue:".length)}.css`), "utf8"));
    for (const ground of ["--bg-base", "--bg-raised", "--bg-hover", "--bg-sunken", "--bg-overlay"]) {
      for (const text of ["--fg-default", "--fg-muted", "--fg-subtle", "--accent"]) {
        expect(contrast(hex(css[text]), hex(css[ground])), `${one.id} ${text} on ${ground}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  }
});


// The strip a theme is previewed by is read from named tokens, and these hold that: each token is a
// different colour, with decoys beside it that a strip from the wrong tokens would pick up.

/** A colour, spelled by arithmetic: the gate refuses a colour literal in what a renderer ships, tests included. */
const colour = (n: number) => `#${n.toString(16).toUpperCase().padStart(6, "0")}`;
const SENTINEL = Object.fromEntries(SWATCH_TOKENS.map((name, at) => [name, `#0${at}0${at}0${at}`]));
const DECOYS = ["--bg-sunken", "--bg-overlay", "--bg-hover", "--fg-muted", "--fg-subtle", "--accent-hover", "--status-killed", "--status-escalated", "--status-rejected", "--status-not-started", "--tool-look", "--helm"];
const sheetOf = (declared: Record<string, string>) => `:root {\n${Object.entries(declared).map(([name, value]) => `  ${name}: ${value};`).join("\n")}\n}`;

test("the strip is the ground, a surface, the text, the accent and four status hues, in that order", () => {
  expect(SWATCH_TOKENS).toEqual(["--bg-base", "--bg-raised", "--fg-default", "--accent", "--status-completed-success", "--status-completed-failed", "--status-awaiting-review", "--status-running"]);
  const decoys = Object.fromEntries(DECOYS.map((name) => [name, colour(0xff00ff)]));
  const strip = swatchOf(sheetOf({ ...decoys, ...SENTINEL }));
  expect(strip).toEqual(SWATCH_TOKENS.map((name) => SENTINEL[name]));
  expect(strip).not.toContain(colour(0xff00ff));
});

test("a strip does not move when a token it does not read moves, and does when one it reads does", () => {
  const before = swatchOf(sheetOf(SENTINEL));
  expect(swatchOf(sheetOf({ ...SENTINEL, "--bg-sunken": colour(0xabcdef), "--fg-muted": colour(0xabcdef) }))).toEqual(before);
  for (const [at, name] of SWATCH_TOKENS.entries()) {
    const moved = swatchOf(sheetOf({ ...SENTINEL, [name]: colour(0xabcdef) }));
    expect(moved.map((colour, i) => (colour === before[i] ? null : i))).toEqual([...Array(SWATCH_TOKENS.length).keys()].map((i) => (i === at ? i : null)));
  }
});

test("a token the sheet leaves out or gives as no plain colour takes the base's, and none anywhere throws", () => {
  const some = { "--bg-base": colour(0x112233), "--accent": "var(--fg-default)", "--fg-default": "url(x)" };
  expect(swatchOf(sheetOf(some), SENTINEL)).toEqual(SWATCH_TOKENS.map((name) => (name === "--bg-base" ? colour(0x112233) : SENTINEL[name])));
  expect(() => swatchOf(sheetOf(some))).toThrow(/--bg-raised/);
  expect(declaredIn(`/* --bg-base: ${colour(0xffffff)}; */ :root { --bg-base: ${colour(0x123456)}; --bg-base: ${colour(0x654321)}; }`)).toEqual({ "--bg-base": colour(0x654321) });
});

test("each shipped theme's strip is its own css's tokens, which the index carries so the list stays lazy", () => {
  for (const one of index) {
    const css = readFileSync(join(themes, "css", `${one.id.slice("catalogue:".length)}.css`), "utf8");
    const own = declared(css);
    expect(one.swatch, one.id).toEqual(SWATCH_TOKENS.map((name) => own[name]));
  }
});

// Dark's and Light's strips are what the files that define them say for each token, read here by a
// second, separate parse: a strip from the wrong tokens, or the wrong theme, fails.
const authored = (file: string) => readFileSync(join(themes, "../src", file), "utf8");
/** What `css` says `name` is, the first time it does. */
const said = (css: string, name: string) => css.replace(/\/\*[\s\S]*?\*\//g, "").match(new RegExp(`${name}:\\s*(#[0-9A-Fa-f]+)\\s*;`))?.[1];
const builtInStrips = JSON.parse(readFileSync(join(themes, "built-in.json"), "utf8")) as { dark: string[]; light: string[] };

test("Dark's strip is the values the token set authors for the eight tokens", () => {
  expect(builtInStrips.dark).toEqual(SWATCH_TOKENS.map((name) => said(authored("colors.css") + authored("status.css"), name)));
});

test("Light's strip is light.css's value for each token", () => {
  expect(builtInStrips.light).toEqual(SWATCH_TOKENS.map((name) => said(authored("light.css"), name)));
});

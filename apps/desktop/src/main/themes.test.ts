// The shipped themes: what is checked in is what the generator writes, every one reads, and each
// overrides only tokens that exist. Node, no document.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

import { contrast, parseHex } from "@armada/tokens/themes/derive.mjs";
import { build, popular } from "@armada/tokens/themes/generate.mjs";

const themes = join(dirname(fileURLToPath(import.meta.url)), "../../../../packages/tokens/themes");
const built = build(popular());
const index = JSON.parse(readFileSync(join(themes, "index.json"), "utf8")) as { id: string; title: string; tone: string; flags: string[] }[];

/** `--name: value` pairs of a theme's one block. */
const declared = (css: string) => Object.fromEntries([...css.matchAll(/^\s+(--\w[\w-]*): ([^;]+);/gm)].map((m) => [m[1]!, m[2]!]));
const hex = (value: string | undefined) => parseHex(value ?? "");

test("what is checked in is what the generator writes", () => {
  expect(built.problems).toEqual([]);
  for (const [path, text] of built.files) expect(readFileSync(join(themes, path), "utf8"), path).toBe(text);
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

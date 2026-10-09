// A terminal colour scheme in, a Bridge theme out: `[data-theme="<id>"]` over the semantic tokens.
//
// Pure, so the generator writes files with it and a test asserts the checked-in ones are what it
// would write. The mapping is documented in docs/concepts/themes.md; the comments here say why a
// number is what it is.
//
// A scheme carries 16 palette colours, a background and a foreground. Bridge's semantic tokens are
// a ground scale (8), a foreground scale (4), an accent (3), depth, a status hue per Job state,
// diff, tool and Helm colours. Everything below is derived from the 18, and every text pair is
// measured; a colour that fails is moved along its own lightness until it passes, and a pair that
// cannot pass is flagged and the theme is not shipped.

const TEXT = 4.5; // WCAG AA for body text, the contract's floor for every text token

// --- colour -----------------------------------------------------------------------------------

export const parseHex = (hex) => {
  const h = hex.trim().replace(/^#/, "");
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
};
const clamp = (x) => Math.min(1, Math.max(0, x));
export const toHex = (c) => "#" + c.map((x) => Math.round(clamp(x) * 255).toString(16).padStart(2, "0")).join("").toUpperCase();

const lin = (v) => (v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
const gamma = (v) => (v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055);

export const luminance = (c) => {
  const [r, g, b] = c.map(lin);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
export const contrast = (a, b) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

const toLab = (c) => {
  const [r, g, b] = c.map(lin);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s];
};
/** Every colour that leaves a function here is one a stylesheet can say, so what is measured is what is painted. */
const eightBit = (c) => c.map((v) => Math.round(v * 255) / 255);
const fromLab = ([L, a, b]) => eightBit(fromLabExact([L, a, b]));
const fromLabExact = ([L, a, b]) => {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s].map((v) => clamp(gamma(clamp(v))));
};

/** `t` of the way from `a` to `b`, in OKLab, so a mix keeps its hue instead of going muddy. */
export const mix = (a, b, t) => {
  const [x, y] = [toLab(a), toLab(b)];
  return fromLab(x.map((v, i) => v + (y[i] - v) * t));
};
/** `top` at `alpha` over `ground`, in sRGB: what `color-mix(in srgb, …, transparent)` paints. */
const over = (top, ground, alpha) => eightBit(top.map((v, i) => v * alpha + ground[i] * (1 - alpha)));

const WHITE = [1, 1, 1];
const BLACK = [0, 0, 0];

/** The lowest contrast of `c` as text across `grounds`; `tint` lays it at 12% over each first. */
const worst = (c, grounds, tint) => Math.min(...grounds.map((g) => contrast(c, tint ? over(c, g, 0.12) : g)));

/** `c` with its lightness `t` of the way to `end`'s and its hue and chroma kept, so a blue stays blue. */
const lighten = (c, end, t) => {
  const [L, a, b] = toLab(c);
  return fromLab([L + (toLab(end)[0] - L) * t, a, b]);
};

/** `c` moved toward `end` in lightness until it reads on every ground, and whether it had to move. */
function reach(c, grounds, end, tint = false) {
  if (worst(c, grounds, tint) >= TEXT) return { color: c, moved: false };
  for (let t = 0.02; t <= 1.0001; t += 0.02) {
    const next = lighten(c, end, t);
    if (worst(next, grounds, tint) >= TEXT) return { color: next, moved: true };
  }
  return { color: end, moved: true };
}

// --- scheme -----------------------------------------------------------------------------------

/** A Ghostty scheme file: `palette = N=#hex`, `background`, `foreground`, and the rest ignored. */
export function parseScheme(text) {
  const palette = [];
  const set = {};
  for (const line of text.split("\n")) {
    const m = line.match(/^\s*([a-z-]+)\s*=\s*(.+?)\s*$/);
    if (m === null) continue;
    if (m[1] === "palette") {
      const p = m[2].match(/^(\d+)=(#[0-9a-fA-F]{6})$/);
      if (p !== null) palette[Number(p[1])] = parseHex(p[2]);
    } else if (/^#[0-9a-fA-F]{6}$/.test(m[2])) set[m[1]] = parseHex(m[2]);
  }
  if (palette.length < 16 || palette.includes(undefined) || set.background === undefined || set.foreground === undefined) throw new Error("a scheme needs 16 palette colours, a background and a foreground");
  return { palette, background: set.background, foreground: set.foreground };
}

/** Light or dark, by the background alone: above the luminance where black and white text tie. */
export const toneOf = (scheme) => (luminance(scheme.background) > 0.179 ? "light" : "dark");

const rgba = (c, a) => `rgb(${c.map((v) => Math.round(v * 255)).join(" ")} / ${a})`;

/**
 * The theme: its CSS, and what was measured. `flags` are failures a person should see, and a theme
 * with any is not shipped; `adjusted` are colours moved to pass, which is the generator working.
 */
export function derive(id, title, scheme) {
  const { palette: p, background: bg, foreground: fg } = scheme;
  const tone = toneOf(scheme);
  const dark = tone === "dark";
  const flags = [];
  const adjusted = [];

  // Ground. A dark scheme lifts toward its own foreground one step at a time; a light one keeps
  // its background as the canvas and lifts cards to white, as the shipped Light does.
  const ground = dark
    ? { base: bg, sunken: mix(bg, BLACK, 0.35), raised: mix(bg, fg, 0.04), overlay: mix(bg, fg, 0.075), hover: mix(bg, fg, 0.105), subtle: mix(bg, fg, 0.1), edge: mix(bg, fg, 0.17), strong: mix(bg, fg, 0.27) }
    : { base: bg, sunken: mix(bg, fg, 0.035), raised: mix(bg, WHITE, 0.75), overlay: mix(bg, WHITE, 0.75), hover: mix(bg, fg, 0.055), subtle: mix(bg, fg, 0.07), edge: mix(bg, fg, 0.14), strong: mix(bg, fg, 0.28) };
  const grounds = [ground.base, ground.sunken, ground.raised, ground.overlay, ground.hover];
  const toward = dark ? WHITE : BLACK; // the way a colour moves to gain contrast

  // Foreground. Some schemes keep their text soft on purpose (Solarized's is 4.1:1), so it is moved
  // to the floor rather than refused; one that starts under 3:1 on its own background is a scheme
  // nobody could read, and is flagged instead.
  if (contrast(fg, bg) < 3) flags.push(`foreground ${contrast(fg, bg).toFixed(2)}:1 on its own background`);
  const { color: fgDefault, moved: fgMoved } = reach(fg, grounds, toward);
  if (fgMoved) adjusted.push("foreground");
  const step = (t) => {
    for (let k = t; k >= 0; k -= 0.02) {
      const c = mix(fgDefault, bg, k);
      if (worst(c, grounds, false) >= TEXT) return c;
    }
    return fgDefault;
  };
  const fgMuted = step(0.32);
  const fgSubtle = step(0.45);

  // Accent: the first coloured palette colour that reads as text, bright blue first. A grey does
  // not count (Solarized's bright blue is one); with none, blue is moved until it reads.
  const chroma = (c) => Math.hypot(toLab(c)[1], toLab(c)[2]);
  let accent = [12, 4, 6, 14, 5, 13].map((i) => p[i]).find((c) => chroma(c) > 0.06 && worst(c, grounds, false) >= TEXT);
  if (accent === undefined) {
    ({ color: accent } = reach(p[4], grounds, toward));
    adjusted.push("accent");
  }
  const accentHover = mix(accent, fgDefault, 0.15);
  let muted = mix(bg, accent, dark ? 0.22 : 0.16);
  for (let i = 0; i < 8 && contrast(fgDefault, muted) < TEXT; i += 1) muted = mix(muted, bg, 0.25);
  const inverse = [bg, WHITE, BLACK].map((c) => [c, contrast(c, accent)]).sort((a, b) => b[1] - a[1])[0][0];

  // Status. Palette 1 danger, 2 success, 3 warning, 4 info, 5 rejected; escalated sits between
  // danger and warning. Each is read as badge text on its own 12% tint over every ground.
  const status = {};
  const hue = (name, c, tint = true) => {
    const r = reach(c, grounds, toward, tint);
    if (r.moved) adjusted.push(name);
    status[name] = r.color;
    return r.color;
  };
  hue("failed", p[1]);
  hue("success", p[2]);
  hue("review", p[3]);
  hue("running", p[4]);
  hue("escalated", mix(p[1], p[3], 0.5));
  hue("rejected", p[5]);
  hue("not-started", mix(fgDefault, bg, 0.52));
  hue("killed", mix(fgDefault, bg, 0.42));
  hue("handed-in", mix(status.running, status.success, 0.5));

  const helm = reach(mix(accent, p[5], 0.5), grounds, toward, true).color;
  const tool = (c) => reach(c, [ground.sunken, ground.hover], toward).color;

  // Diff text sits on its own tinted ground.
  const diffGround = (c) => over(c, bg, dark ? 0.16 : 0.14);
  const diff = (c) => {
    const tinted = diffGround(c);
    return { bg: tinted, fg: reach(c, [tinted], toward).color };
  };
  const add = diff(p[2]);
  const del = diff(p[1]);

  const v = (name, c) => `  --${name}: ${typeof c === "string" ? c : toHex(c)};`;
  const lines = [
    "  /* Ground */",
    v("bg-base", ground.base), v("bg-sunken", ground.sunken), v("bg-raised", ground.raised), v("bg-overlay", ground.overlay), v("bg-hover", ground.hover),
    v("border-subtle", ground.subtle), v("border-default", ground.edge), v("border-strong", ground.strong),
    "  /* Foreground and accent */",
    v("fg-default", fgDefault), v("fg-muted", fgMuted), v("fg-subtle", fgSubtle), v("fg-inverse", inverse),
    v("accent", accent), v("accent-hover", accentHover), v("accent-muted", muted),
    "  /* Depth */",
    v("bg-glass", rgba(ground.raised, 0.86)), v("bg-glass-end", rgba(ground.base, 0.86)),
    v("border-glass", dark ? "rgb(255 255 255 / 0.07)" : rgba(fgDefault, 0.08)),
    v("border-highlight", dark ? "rgb(255 255 255 / 0.05)" : "rgb(255 255 255 / 0.6)"),
    ...(dark
      ? []
      : [
          v("shadow-overlay", "0 8px 24px -6px rgb(15 23 42 / 0.18), 0 2px 6px -2px rgb(15 23 42 / 0.12)"),
          v("shadow-card", "0 1px 2px rgb(15 23 42 / 0.08), 0 12px 28px -16px rgb(15 23 42 / 0.25)"),
          v("scrim", "color-mix(in srgb, var(--fg-default) 40%, transparent)"),
        ]),
    "  /* Status */",
    ...Object.entries({ "not-started": 0, running: 0, "awaiting-review": 0, escalated: 0, "completed-success": 0, "completed-failed": 0, rejected: 0, killed: 0, "handed-in": 0 }).map(([k]) => {
      const key = { "awaiting-review": "review", "completed-success": "success", "completed-failed": "failed" }[k] ?? k;
      return v(`status-${k}`, status[key]);
    }),
    "  /* Helm, tools, diff */",
    v("helm", helm),
    v("tool-look", tool(p[6])), v("tool-change", tool(p[5])), v("tool-run", tool(p[3])),
    v("diff-add-bg", add.bg), v("diff-add-fg", add.fg), v("diff-del-bg", del.bg), v("diff-del-fg", del.fg), v("diff-context", fgMuted),
  ];

  const note = [`Derived from the ${title} terminal scheme by packages/tokens/themes/generate.mjs. Do not hand-edit.`, `Tone ${tone}. foreground ${worst(fgDefault, grounds, false).toFixed(1)}:1, accent ${worst(accent, grounds, false).toFixed(1)}:1 at the worst ground.`];
  if (adjusted.length > 0) note.push(`Lightness moved to reach ${TEXT}:1: ${adjusted.join(", ")}.`);
  const css = `/* ${note.join("\n   ")} */\n[data-theme="${id}"] {\n  color-scheme: ${tone};\n${lines.join("\n")}\n}\n`;
  return { css, tone, flags, adjusted };
}

/** The id a theme is chosen by and the file it lives in. Prefixed so no mod's name can collide. */
export const slugOf = (name) => name.toLowerCase().replace(/\+$/, "").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
export const idOf = (name) => `catalogue:${slugOf(name)}`;

/** Names as the site and the tools show them: `Dracula+` is Dracula, the way cmux lists it. */
export const titleOf = (name) => name.replace(/^iTerm2 /, "").replace(/\+$/, "").replace(/^TokyoNight/, "Tokyo Night").replace(/^Rose Pine/, "Rosé Pine");

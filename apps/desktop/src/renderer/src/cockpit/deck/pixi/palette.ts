// The map's colours and type, read from the tokens the page is wearing rather than written here. A canvas cannot see a CSS variable, so
// each is resolved the way the page resolves it: set on a probe element and read back as a computed colour, which takes in a theme's
// override and a `color-mix()` alike. `readPalette` is the one place that touches the page; a theme change is a second call to it and
// a redraw, which the stage makes when the page's `data-theme` changes.

/** Red, green, blue and alpha, each 0 to 1. */
export type Rgba = readonly [number, number, number, number];

export type Palette = {
  base: Rgba;
  fg: Rgba;
  muted: Rgba;
  subtle: Rgba;
  accent: Rgba;
  /** Needs him, or stuck. */
  ask: Rgba;
  /** A Check failing. */
  bad: Rgba;
  /** Landed, green. */
  good: Rgba;
  /** In flight. */
  run: Rgba;
  /** Being written, or at work. */
  hot: Rgba;
  /** Not started. */
  dormant: Rgba;
  /** Looking: a file read. The tool family's own hue, which carries no status. */
  look: Rgba;
  mono: string;
  /** The size of the smallest type, in pixels. */
  small: number;
  /** How long the camera takes to ease, a plot to rise, and a loop to turn once, in milliseconds. All 0 where he has asked for no motion. */
  camMs: number;
  riseMs: number;
  pulseMs: number;
};

let scratch: CanvasRenderingContext2D | undefined;

/** A computed CSS colour, whatever syntax it came back in, as 0 to 1 channels. */
function rgbaOf(css: string): Rgba {
  scratch ??= document.createElement("canvas").getContext("2d", { willReadFrequently: true })!;
  scratch.canvas.width = 1;
  scratch.canvas.height = 1;
  scratch.clearRect(0, 0, 1, 1);
  scratch.fillStyle = "#000";
  scratch.fillStyle = css;
  scratch.fillRect(0, 0, 1, 1);
  const [r, g, b, a] = scratch.getImageData(0, 0, 1, 1).data;
  // The canvas keeps colour premultiplied, so a translucent token reads back darker; undo it.
  const alpha = a! / 255;
  return alpha === 0 ? [0, 0, 0, 0] : [r! / 255 / alpha, g! / 255 / alpha, b! / 255 / alpha, alpha];
}

const seconds = (css: string): number => (css.endsWith("ms") ? parseFloat(css) : parseFloat(css) * 1000);

export function readPalette(host: HTMLElement): Palette {
  const probe = document.createElement("span");
  host.append(probe);
  const colour = (name: string): Rgba => {
    probe.style.color = "";
    probe.style.color = `var(${name})`;
    return rgbaOf(getComputedStyle(probe).color);
  };
  const motion = (expression: string): number => {
    probe.style.transitionDuration = expression;
    return seconds(getComputedStyle(probe).transitionDuration.split(",")[0]!.trim());
  };
  probe.style.fontFamily = "var(--font-mono)";
  probe.style.fontSize = "var(--text-2xs)";
  const type = getComputedStyle(probe);
  const mono = type.fontFamily;
  const small = parseFloat(type.fontSize) || 13;
  // The stylesheet turns these transitions off itself under reduced motion; a canvas has to ask.
  const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const palette: Palette = {
    base: colour("--bg-base"),
    fg: colour("--fg-default"),
    muted: colour("--fg-muted"),
    subtle: colour("--fg-subtle"),
    accent: colour("--accent"),
    ask: colour("--status-awaiting-review"),
    bad: colour("--status-completed-failed"),
    good: colour("--status-completed-success"),
    run: colour("--status-running"),
    hot: colour("--status-escalated"),
    dormant: colour("--status-not-started"),
    look: colour("--tool-look"),
    mono,
    small,
    camMs: still ? 0 : motion("calc(var(--duration-pulse) * 0.7)"),
    riseMs: still ? 0 : motion("calc(var(--duration-pulse) * 0.45)"),
    pulseMs: still ? 0 : motion("var(--duration-pulse)"),
  };
  probe.remove();
  return palette;
}

/** `color-mix(in srgb, a (1 - t), b t)`: the same blend the stylesheet writes. */
export const mix = (a: Rgba, b: Rgba, t: number): Rgba => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t];

/** The colour at `t` of the way from the ground to `over`, opaque: how the CSS lays `color-mix(over t%, base)`. */
export const onGround = (p: Palette, over: Rgba, t: number): Rgba => mix(p.base, over, t);

/** The same colour at a fraction of its alpha: `color-mix(c t%, transparent)`. */
export const fade = (c: Rgba, t: number): Rgba => [c[0], c[1], c[2], c[3] * t];

/** A colour for a Pixi display object: 0xRRGGBB. */
export const hex = (c: Rgba): number => (Math.round(c[0] * 255) << 16) | (Math.round(c[1] * 255) << 8) | Math.round(c[2] * 255);

// The two soft pictures every light on the map is made from, drawn once in white and tinted where they are used: a round glow that
// falls off to nothing, and a pillar that is bright at its foot, faint at its head and soft at both edges. Added to what is under
// them, they brighten it, which is what a beam or a flare does and a flat fill cannot.

import { Sprite, Texture } from "pixi.js";

import { hex, type Rgba } from "./palette";

let disc: Texture | undefined;
let pillar: Texture | undefined;

function canvasOf(width: number, height: number, alphaAt: (x: number, y: number) => number): Texture {
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  const image = ctx.createImageData(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const at = (y * width + x) * 4;
      image.data[at] = 255;
      image.data[at + 1] = 255;
      image.data[at + 2] = 255;
      image.data[at + 3] = Math.round(255 * alphaAt(x / (width - 1), y / (height - 1)));
    }
  }
  ctx.putImageData(image, 0, 0);
  return Texture.from(canvas);
}

/** A soft round glow, 1 across: full at the middle, nothing at the edge, eased so it has no visible rim. */
function glowTexture(): Texture {
  disc ??= canvasOf(64, 64, (x, y) => {
    const t = Math.min(1, Math.hypot(x - 0.5, y - 0.5) * 2);
    return (1 - t) * (1 - t) * (1 - t * 0.5);
  });
  return disc;
}

/** A pillar, 1 wide and 1 tall with its foot at the bottom: `0.62` at the foot, `0.24` at six tenths of the way up, nothing at the head. */
function pillarTexture(): Texture {
  pillar ??= canvasOf(16, 64, (x, y) => {
    const side = x < 0.38 ? x / 0.38 : x > 0.62 ? (1 - x) / 0.38 : 1;
    const up = 1 - y;
    const along = up < 0.6 ? 0.62 + (0.24 - 0.62) * (up / 0.6) : 0.24 * (1 - (up - 0.6) / 0.4);
    return side * along;
  });
  return pillar;
}

/** A round glow of `radius` pixels, centred on its origin, in `tone`, added to what is under it. */
export function glowSprite(radius: number, tone: Rgba, strength = 1): Sprite {
  const sprite = new Sprite(glowTexture());
  sprite.anchor.set(0.5);
  sprite.width = radius * 2;
  sprite.height = radius * 2;
  sprite.tint = hex(tone);
  sprite.alpha = tone[3] * strength;
  sprite.blendMode = "add";
  return sprite;
}

/** A pillar of light, anchored at its foot, `width` wide and `height` tall in pixels. */
export function pillarSprite(width: number, height: number, tone: Rgba): Sprite {
  const sprite = new Sprite(pillarTexture());
  sprite.anchor.set(0.5, 1);
  sprite.width = width;
  sprite.height = height;
  sprite.tint = hex(tone);
  sprite.alpha = tone[3];
  sprite.blendMode = "add";
  return sprite;
}

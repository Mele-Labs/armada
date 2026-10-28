// The ground and what lies on it, as triangles the graphics card projects. A shape is built once in the plane's own units, where it is
// x, depth and height, and the vertex shader turns every vertex into a pixel with the camera's turn, scale and perspective, so a pan or a
// zoom is a few uniforms and not a redraw. A stroke is a ribbon that keeps a width on the screen: the shader steps each of its vertices
// out along the stroke's normal by that many pixels, so a line stays crisp at any zoom and never thins below a hairline.
//
// Triangles are drawn in the order they are added, which is the painter's order: far to near, a plot's sides before its top.

import { Buffer, BufferUsage, Geometry, GlProgram, Mesh, Shader, UniformGroup } from "pixi.js";

import type { P3 } from "../layout";
import { packView, type View } from "./project";
import type { Rgba } from "./palette";

const VERTEX = /* glsl */ `
in vec3 aPosition;
in vec4 aColor;
in vec3 aEdge;

uniform mat3 uProjectionMatrix;
uniform mat3 uWorldTransformMatrix;
uniform mat3 uTransformMatrix;
uniform vec2 uCam;
uniform float uEye;
uniform vec4 uTrig;
uniform vec3 uScreen;
uniform float uHairline;

out vec4 vColor;

vec3 toScreen(vec3 p) {
  vec3 d = vec3((p.xy - uCam) * uEye, p.z * uEye);
  float ux = d.x * uTrig.x - d.y * uTrig.y;
  float uy = d.x * uTrig.y + d.y * uTrig.x;
  float s = uScreen.z / (uScreen.z - (uy * uTrig.w + d.z * uTrig.z));
  return vec3(uScreen.xy + vec2(ux, uy * uTrig.z - d.z * uTrig.w) * s, s);
}

void main() {
  vec3 at = toScreen(aPosition);
  vec2 pixel = at.xy;
  if (aEdge.z != 0.0) {
    vec2 along = toScreen(aPosition + vec3(aEdge.xy, 0.0)).xy - at.xy;
    float reach = length(along);
    float width = max(abs(aEdge.z) * uEye * at.z, uHairline);
    if (reach > 0.00001) pixel += along / reach * width * sign(aEdge.z);
  }
  mat3 mvp = uProjectionMatrix * uWorldTransformMatrix * uTransformMatrix;
  gl_Position = vec4((mvp * vec3(pixel, 1.0)).xy, 0.0, 1.0);
  vColor = aColor;
}
`;

const FRAGMENT = /* glsl */ `
in vec4 vColor;
out vec4 finalColor;
void main() {
  finalColor = vColor;
}
`;

/** Floats to a vertex: position (3), colour (4), and the stroke's normal and half width (3). */
const STRIDE = 10;

/** The most dashes one segment of a stroke is cut into; a longer one is drawn solid, so no stroke can grow without bound. */
const MOST_DASHES = 2000;

const program = () => GlProgram.from({ vertex: VERTEX, fragment: FRAGMENT, name: "bf-plane" });

/** The camera the shaders of every layer read, one group shared so a move is written once. */
export class PlaneView {
  readonly group: UniformGroup;
  private readonly packed = { cam: new Float32Array(2), trig: new Float32Array(4), screen: new Float32Array(3) };

  constructor() {
    this.group = new UniformGroup({
      uCam: { value: this.packed.cam, type: "vec2<f32>" },
      uEye: { value: 1, type: "f32" },
      uTrig: { value: this.packed.trig, type: "vec4<f32>" },
      uScreen: { value: this.packed.screen, type: "vec3<f32>" },
      uHairline: { value: 0.5, type: "f32" },
    });
  }

  set(view: View): void {
    packView(view, this.packed);
    this.group.uniforms.uEye = view.eye;
    this.group.update();
  }
}

/** Triangles for one layer, collected in drawing order. */
export class PlaneBuilder {
  private readonly verts: number[] = [];
  private readonly index: number[] = [];
  private count = 0;

  get empty(): boolean {
    return this.count === 0;
  }

  vertex(x: number, y: number, z: number, c: Rgba, nx = 0, ny = 0, half = 0): number {
    const a = c[3];
    // Premultiplied, which is how the canvas is composited.
    this.verts.push(x, y, z, c[0] * a, c[1] * a, c[2] * a, a, nx, ny, half);
    return this.count++;
  }

  tri(a: number, b: number, c: number): void {
    this.index.push(a, b, c);
  }

  /** A convex polygon, filled. */
  fan(points: readonly P3[], c: Rgba): void {
    if (points.length < 3) return;
    const first = this.count;
    for (const p of points) this.vertex(p.x, p.y, p.z, c);
    for (let i = 1; i < points.length - 1; i += 1) this.tri(first, first + i, first + i + 1);
  }

  /** A convex polygon whose corners each have a colour. */
  shaded(points: readonly P3[], colours: readonly Rgba[]): void {
    if (points.length < 3) return;
    const first = this.count;
    points.forEach((p, i) => this.vertex(p.x, p.y, p.z, colours[i]!));
    for (let i = 1; i < points.length - 1; i += 1) this.tri(first, first + i, first + i + 1);
  }

  /** The same rectangle, for the many a plot holds: no points are made to describe it. */
  quad(x: number, y: number, w: number, h: number, z: number, c: Rgba): void {
    const first = this.count;
    this.vertex(x, y, z, c);
    this.vertex(x + w, y, z, c);
    this.vertex(x + w, y + h, z, c);
    this.vertex(x, y + h, z, c);
    this.index.push(first, first + 1, first + 2, first, first + 2, first + 3);
  }

  /** An axis-aligned rectangle lying flat at height z. */
  flat(x: number, y: number, w: number, h: number, z: number, c: Rgba): void {
    this.fan([{ x, y, z }, { x: x + w, y, z }, { x: x + w, y: y + h, z }, { x, y: y + h, z }], c);
  }

  /**
   * A line of the given half width (in plane units, kept at least a hairline on the screen), through `points`, optionally dashed:
   * `dash` is the length of a dash and of the gap after it. Corners are not mitred, so a stroke through a sharp one thins there.
   */
  stroke(points: readonly P3[], half: number, c: Rgba, opts: { closed?: boolean; dash?: readonly [number, number]; offset?: number; shade?: (p: P3) => Rgba; normal?: { x: number; y: number } } = {}): void {
    const path = opts.closed === true ? [...points, points[0]!] : points;
    if (path.length < 2) return;
    const { shade, normal } = opts;
    if (opts.dash === undefined) {
      this.ribbon(path, half, c, shade, normal);
      return;
    }
    const [on, off] = opts.dash;
    const cycle = on + off;
    let phase = (((opts.offset ?? 0) % cycle) + cycle) % cycle;
    let run: P3[] = [];
    const flush = () => {
      if (run.length >= 2) this.ribbon(run, half, c, shade, normal);
      run = [];
    };
    for (let i = 0; i < path.length - 1; i += 1) {
      const [a, b] = [path[i]!, path[i + 1]!];
      const length = Math.hypot(b.x - a.x, b.y - a.y, b.z - a.z);
      // A segment to a point at infinity would be dashed forever, until the vertices outgrow what an array can hold.
      if (length === 0 || !Number.isFinite(length)) continue;
      if (length / cycle > MOST_DASHES) {
        flush();
        this.ribbon([a, b], half, c, shade, normal);
        continue;
      }
      const lerp = (s: number): P3 => ({ x: a.x + ((b.x - a.x) * s) / length, y: a.y + ((b.y - a.y) * s) / length, z: a.z + ((b.z - a.z) * s) / length });
      let at = 0;
      while (at < length) {
        const dashing = phase < on;
        const step = Math.min((dashing ? on : cycle) - phase, length - at);
        if (dashing) {
          if (run.length === 0) run.push(lerp(at));
          run.push(lerp(at + step));
        }
        at += step;
        phase += step;
        if (phase >= cycle - 1e-9) phase = 0;
        else if (dashing && phase >= on - 1e-9) flush();
      }
    }
    flush();
  }

  private ribbon(path: readonly P3[], half: number, c: Rgba, shade?: (p: P3) => Rgba, fixed?: { x: number; y: number }): void {
    const n = path.length;
    // A normal in the plane at each point: the turn of the line there, the mean of its two sides. A line that stands upright has none
    // of its own, so it is given one.
    const normals = path.map((_, i) => {
      if (fixed !== undefined) return fixed;
      const a = path[Math.max(0, i - 1)]!;
      const b = path[Math.min(n - 1, i + 1)]!;
      const [dx, dy] = [b.x - a.x, b.y - a.y];
      const length = Math.hypot(dx, dy);
      return length === 0 ? { x: 0, y: 1 } : { x: -dy / length, y: dx / length };
    });
    const first = this.count;
    path.forEach((p, i) => {
      const m = normals[i]!;
      const colour = shade === undefined ? c : shade(p);
      this.vertex(p.x, p.y, p.z, colour, m.x, m.y, half);
      this.vertex(p.x, p.y, p.z, colour, m.x, m.y, -half);
    });
    for (let i = 0; i < n - 1; i += 1) {
      const k = first + i * 2;
      this.tri(k, k + 1, k + 2);
      this.tri(k + 1, k + 3, k + 2);
    }
  }

  /** The points of a rounded rectangle on the plane, going round once. */
  static rounded(x: number, y: number, w: number, h: number, r: number, z: number, steps = 6): P3[] {
    const radius = Math.max(0, Math.min(r, w / 2, h / 2));
    const out: P3[] = [];
    const corner = (cx: number, cy: number, from: number) => {
      for (let i = 0; i <= steps; i += 1) {
        const a = from + (i / steps) * (Math.PI / 2);
        out.push({ x: cx + Math.cos(a) * radius, y: cy + Math.sin(a) * radius, z });
      }
    };
    corner(x + w - radius, y + radius, -Math.PI / 2);
    corner(x + w - radius, y + h - radius, 0);
    corner(x + radius, y + h - radius, Math.PI / 2);
    corner(x + radius, y + radius, Math.PI);
    return out;
  }

  /** The points of a circle, or a part of one, on the plane. */
  static arc(cx: number, cy: number, r: number, from: number, to: number, z: number, steps = 32): P3[] {
    return Array.from({ length: steps + 1 }, (_, i) => {
      const a = from + ((to - from) * i) / steps;
      return { x: cx + Math.cos(a) * r, y: cy + Math.sin(a) * r, z };
    });
  }

  /** A filled circle, or a sector of an annulus when `inner` is given, on the plane. */
  disc(cx: number, cy: number, outer: number, c: Rgba, z: number, opts: { inner?: number; from?: number; to?: number; steps?: number } = {}): void {
    const { inner = 0, from = 0, to = Math.PI * 2, steps = Math.max(8, Math.round((Math.abs(to - from) / (Math.PI * 2)) * 40)) } = opts;
    if (inner <= 0) {
      const first = this.count;
      this.vertex(cx, cy, z, c);
      for (let i = 0; i <= steps; i += 1) {
        const a = from + ((to - from) * i) / steps;
        this.vertex(cx + Math.cos(a) * outer, cy + Math.sin(a) * outer, z, c);
      }
      for (let i = 1; i <= steps; i += 1) this.tri(first, first + i, first + i + 1);
      return;
    }
    const first = this.count;
    for (let i = 0; i <= steps; i += 1) {
      const a = from + ((to - from) * i) / steps;
      this.vertex(cx + Math.cos(a) * outer, cy + Math.sin(a) * outer, z, c);
      this.vertex(cx + Math.cos(a) * inner, cy + Math.sin(a) * inner, z, c);
    }
    for (let i = 0; i < steps; i += 1) {
      const k = first + i * 2;
      this.tri(k, k + 1, k + 2);
      this.tri(k + 1, k + 3, k + 2);
    }
  }

  /** A ring from `r0` to `r1` whose colour runs from `c0` at the inner edge to `c1` at the outer: a glow, or a soft edge. */
  halo(cx: number, cy: number, r0: number, r1: number, c0: Rgba, c1: Rgba, z: number, steps = 40): void {
    const first = this.count;
    for (let i = 0; i <= steps; i += 1) {
      const a = (i / steps) * Math.PI * 2;
      this.vertex(cx + Math.cos(a) * r0, cy + Math.sin(a) * r0, z, c0);
      this.vertex(cx + Math.cos(a) * r1, cy + Math.sin(a) * r1, z, c1);
    }
    for (let i = 0; i < steps; i += 1) {
      const k = first + i * 2;
      this.tri(k, k + 1, k + 2);
      this.tri(k + 1, k + 3, k + 2);
    }
  }

  /** The geometry for what was added. */
  geometry(): Geometry {
    const data = new Float32Array(this.verts);
    const buffer = new Buffer({ data, usage: BufferUsage.VERTEX | BufferUsage.COPY_DST });
    const stride = STRIDE * 4;
    return new Geometry({
      attributes: {
        aPosition: { buffer, format: "float32x3", stride, offset: 0 },
        aColor: { buffer, format: "float32x4", stride, offset: 12 },
        aEdge: { buffer, format: "float32x3", stride, offset: 28 },
      },
      indexBuffer: new Uint32Array(this.index),
    });
  }
}

/** One mesh of the ground, drawn with the shared camera. */
export class PlaneLayer {
  readonly mesh: Mesh<Geometry, Shader>;
  private readonly shader: Shader;

  /** `glow` adds its colour to what is under it, so overlapping light brightens and nothing darkens. */
  constructor(view: PlaneView, glow = false) {
    this.shader = new Shader({ glProgram: program(), resources: { view: view.group } });
    // A buffer cannot be empty, so the mesh starts as one invisible triangle and is hidden until it is given some.
    const seed = new PlaneBuilder();
    seed.fan([{ x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 0 }], [0, 0, 0, 0]);
    this.mesh = new Mesh({ geometry: seed.geometry(), shader: this.shader });
    this.mesh.visible = false;
    // The mesh's own bounds are the plane's, not the screen's: it is never culled by them.
    this.mesh.cullable = false;
    this.mesh.eventMode = "none";
    if (glow) this.mesh.state.blendMode = "add";
  }

  set(builder: PlaneBuilder): void {
    this.mesh.visible = !builder.empty;
    if (builder.empty) return;
    const was = this.mesh.geometry;
    this.mesh.geometry = builder.geometry();
    was.destroy(true);
  }

  destroy(): void {
    // The mesh lets go of its geometry as it goes, so it is held first.
    const { geometry } = this.mesh;
    this.mesh.destroy({ children: true });
    geometry.destroy(true);
    this.shader.destroy();
  }
}

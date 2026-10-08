// Pre-drawn sketches for the mock: what a mermaid renderer would hand back for the source beside
// each one. **Mock only.** No renderer is in the repository yet, so a sketch carries its drawing;
// the classes (`sk-node`, `sk-edge`, `sk-text`) are styled by `SketchPane.css`.

export type SketchNode = { id: string; x: number; y: number; label: string };

const W = 150;
const H = 44;

/** Boxes joined by arrows, each arrow from one box's nearest edge to the other's. */
export function sketchSvg(nodes: readonly SketchNode[], edges: readonly (readonly [string, string])[]): string {
  const at = (id: string): SketchNode => nodes.find((one) => one.id === id) ?? nodes[0]!;
  const width = Math.max(...nodes.map((one) => one.x)) + W + 20;
  const height = Math.max(...nodes.map((one) => one.y)) + H + 20;
  const lines = edges.map(([from, to]) => {
    const a = at(from);
    const b = at(to);
    const across = Math.abs(b.x - a.x) >= Math.abs(b.y - a.y);
    const [x1, y1, x2, y2] = across
      ? [b.x > a.x ? a.x + W : a.x, a.y + H / 2, b.x > a.x ? b.x : b.x + W, b.y + H / 2]
      : [a.x + W / 2, b.y > a.y ? a.y + H : a.y, b.x + W / 2, b.y > a.y ? b.y : b.y + H];
    return `<line class="sk-edge" x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" marker-end="url(#sk-head)"/>`;
  });
  const boxes = nodes.map(
    (one) =>
      `<rect class="sk-node" x="${one.x}" y="${one.y}" width="${W}" height="${H}" rx="6"/>` +
      `<text class="sk-text" x="${one.x + W / 2}" y="${one.y + H / 2}">${one.label}</text>`,
  );
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} ${height}" width="${width}" height="${height}">` +
    `<defs><marker id="sk-head" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path class="sk-head" d="M0 0L10 5L0 10z"/></marker></defs>` +
    lines.join("") +
    boxes.join("") +
    `</svg>`
  );
}

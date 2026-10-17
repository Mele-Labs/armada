// A Session's small plus quad, drawn flat in SVG for the Build column's subagents. The map draws its drones itself, on the canvas
// (`pixi/drones.ts`); this is the one place the picture is still markup, and it holds still.

/** One rotor, drawn as a squashed disc so it reads at an angle; its blades sit inside it. */
function Rotor({ x, y, r }: { x: number; y: number; r: number }) {
  return (
    <>
      <g transform={`translate(${x} ${y}) scale(1 0.38)`}>
        <circle className="bf-disc" r={r} />
        <g className="bf-blades">
          <line x1={-r} x2={r} vectorEffect="non-scaling-stroke" />
          <line y1={-r} y2={r} vectorEffect="non-scaling-stroke" />
        </g>
      </g>
      <circle className="bf-hub" cx={x} cy={y} r={2} />
    </>
  );
}

/** A Session: the small plus quad, a round pod between four short arms. */
export function SessionGlyph() {
  return (
    <g className="bf-glyph" data-kind="session">
      <path className="bf-arm" d="M-13,0 L-4,0 M13,0 L4,0 M0,-8 L0,-3 M0,8 L0,3" />
      <Rotor x={-15} y={0} r={7} />
      <Rotor x={15} y={0} r={7} />
      <Rotor x={0} y={-10} r={6} />
      <circle className="bf-hull" r={6.2} />
      <circle className="bf-plate" r={3.2} />
      <circle className="bf-led" r={1.4} />
      <Rotor x={0} y={10} r={7} />
    </g>
  );
}

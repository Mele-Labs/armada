/**
 * A group's shape, drawn rather than named: dots on a line for tasks that run
 * one after another, and a fan out to a join for tasks that run together.
 *
 * **The owner sketched it on 29 Sep 2026**, after three passes at saying it in
 * words — `2 tasks, one after another`, then a `sequential` chip. Each was a
 * label for something the reader has to picture anyway.
 *
 * **A diagram, not a canvas.** Nothing here is pressable and nothing is laid
 * out; React Flow draws the plan's real graph on the Plan destination.
 */

/** Where the opening bar stands, and half its height. */
const START = 2;
const CAP = 6;
/** Half the boundary diamond, point to point. */
const GATE = 6;
const NODE = 4;
const ROW = 11;
/** One task to the next, and the gap the gate gets after the last of them. */
const STEP = 17;
const HEIGHT = 34;
const MID = HEIGHT / 2;

/** Past this the dots stop being countable and become texture. */
const MOST = 5;

export type GroupShapeProps = {
  /** How many tasks the group holds. Drawn up to five, then elided. */
  tasks: number;
  /** Whether they run at the same time. */
  concurrent: boolean;
  /** Said to somebody who cannot see it — the caller's words, not composed here. */
  label: string;
};

export function GroupShape({ tasks, concurrent, label }: GroupShapeProps) {
  const drawn = Math.min(tasks, MOST);
  // **The gate gets a step of its own past the last task.** Sized off the
  // marks rather than off the frame: the diamond sat on the last circle when
  // it was placed by subtracting from the width.
  const last = concurrent ? STEP * 3 : STEP * (drawn + 1);
  const width = last + GATE + START;

  return (
    <svg
      className="armada-group-shape"
      viewBox={`0 0 ${width} ${HEIGHT}`}
      width={width}
      height={HEIGHT}
      role="img"
      aria-label={label}
    >
      {concurrent ? (
        <ConcurrentShape drawn={drawn} last={last} />
      ) : (
        <SequentialShape drawn={drawn} last={last} />
      )}
      {/* **Neither end is a circle**, because a circle is what a task is —
          three round marks in a row read as three tasks (the owner, 29 Sep
          2026). The group opens on a bar, the way a track starts, and closes
          on the diamond a gate is drawn as: a group's end is its boundary,
          where the Checks run. */}
      <line
        className="armada-group-shape__start"
        x1={START}
        y1={MID - CAP}
        x2={START}
        y2={MID + CAP}
      />
      <path
        className="armada-group-shape__gate"
        d={`M ${last} ${MID - GATE} L ${last + GATE} ${MID} L ${last} ${MID + GATE} L ${last - GATE} ${MID} Z`}
      />
    </svg>
  );
}

/** Tasks in a line, the boundary at the end of it. */
function SequentialShape({ drawn, last }: { drawn: number; last: number }) {
  return (
    <>
      <line className="armada-group-shape__edge" x1={START} y1={MID} x2={last} y2={MID} />
      {Array.from({ length: drawn }, (_, at) => (
        <circle
          key={at}
          className="armada-group-shape__task"
          cx={STEP * (at + 1)}
          cy={MID}
          r={NODE}
        />
      ))}
    </>
  );
}

/**
 * Tasks side by side between one start and one join.
 *
 * **Spread around the middle**, so the fan is symmetrical however many it
 * holds — drawn from the top edge it leans, and a leaning fan reads as a
 * mistake rather than a shape.
 */
function ConcurrentShape({ drawn, last }: { drawn: number; last: number }) {
  const spread = (drawn - 1) / 2;
  return (
    <>
      {Array.from({ length: drawn }, (_, at) => {
        const y = MID + (at - spread) * ROW;
        return (
          <g key={at}>
            <path
              className="armada-group-shape__edge"
              d={`M ${START} ${MID} Q ${STEP} ${y} ${STEP * 1.5} ${y}`}
              fill="none"
            />
            <path
              className="armada-group-shape__edge"
              d={`M ${STEP * 1.5} ${y} Q ${STEP * 2} ${y} ${last} ${MID}`}
              fill="none"
            />
            <circle className="armada-group-shape__task" cx={STEP * 1.5} cy={y} r={NODE} />
          </g>
        );
      })}
    </>
  );
}

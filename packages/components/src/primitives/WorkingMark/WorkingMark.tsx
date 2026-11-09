import { useLayoutEffect, useRef } from "react";

/**
 * WorkingMark — the icon that says a job is working, or a place is waiting on
 * data: a ring with a dot orbiting it. `currentColor`, so it takes the ground it sits on.
 */
export type WorkingMarkProps = {
  size?: number;
  className?: string;
};

export function WorkingMark({ size = 14, className }: WorkingMarkProps) {
  const spin = useRef<SVGGElement>(null);
  // Every mark turns on one phase, the timeline's own. A mark drawn afresh where another stood, as a new call's line takes
  // it from the last, carries on where that one was instead of starting again from the top, which read as a skip.
  useLayoutEffect(() => {
    for (const one of spin.current?.getAnimations?.() ?? []) one.startTime = 0;
  }, []);
  return (
    <span
      className={className ? `armada-working-mark ${className}` : "armada-working-mark"}
      aria-hidden
    >
      <svg
        width={size}
        height={size}
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      >
        <circle cx="8" cy="8" r="6" opacity="0.3" />
        <g ref={spin} className="armada-working-mark__spin">
          <circle cx="8" cy="2" r="1.4" fill="currentColor" stroke="none" />
        </g>
      </svg>
    </span>
  );
}

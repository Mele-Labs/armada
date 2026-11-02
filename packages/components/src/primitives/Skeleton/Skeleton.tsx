import { WorkingMark } from "../WorkingMark/WorkingMark";
import type { HTMLAttributes } from "react";

/**
 * Skeleton — the shape of data that has not arrived, drawn as a row of
 * telemetry cells in a wash of the foreground on the surface it will land on;
 * see Skeleton.css.
 *
 * **It does not shimmer or sweep.** The motion is a stepped readout lighting
 * cell by cell, never a travelling highlight, and none under reduced motion.
 */
export type SkeletonProps = HTMLAttributes<HTMLDivElement> & {
  /**
   * How wide the bar runs, as a CSS length. Defaults to the full width of its
   * container. Varying it across lines is what stops a block reading as a
   * table; there is no token for it because a placeholder's width is a
   * property of the string it stands in for.
   */
  width?: string;
};

export function Skeleton({ className, width, style, ...rest }: SkeletonProps) {
  return (
    <div
      className={className ? `armada-skeleton ${className}` : "armada-skeleton"}
      aria-hidden
      style={width ? { ...style, width } : style}
      {...rest}
    />
  );
}

/**
 * A block of them, for a paragraph or a row that has not loaded. `label` is
 * what a screen reader is told, because the bars themselves say nothing —
 * without it the region is silent rather than pending.
 */
export type SkeletonTextProps = HTMLAttributes<HTMLDivElement> & {
  widths?: string[];
  label?: string;
};

const DEFAULT_WIDTHS = ["60%", "85%", "40%"];

export function SkeletonText({
  className,
  widths = DEFAULT_WIDTHS,
  label = "Loading",
  ...rest
}: SkeletonTextProps) {
  return (
    <div
      className={className ? `armada-skeleton-text ${className}` : "armada-skeleton-text"}
      role="status"
      aria-label={label}
      aria-busy
      {...rest}
    >
      <WorkingMark className="armada-skeleton-text__mark" />
      {widths.map((w, i) => (
        <Skeleton key={`${w}-${i}`} width={w} />
      ))}
    </div>
  );
}

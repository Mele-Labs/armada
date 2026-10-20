import type { InputHTMLAttributes } from "react";

/**
 * One of a few ordered steps, moved by dragging or by the arrow keys. The step's name is drawn at the
 * end of the track and read out as the value, so a step is never a bare number.
 */
export type SliderProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "value" | "onChange" | "min" | "max" | "step"> & {
  /** The steps, lowest first. */
  steps: readonly string[];
  value: string;
  onChange: (value: string) => void;
  /** The words a step is drawn as, where its own name is not them. */
  show?: (step: string) => string;
};

export function Slider({ steps, value, onChange, show = (step) => step, ...rest }: SliderProps) {
  const at = Math.max(0, steps.indexOf(value));
  const shown = show(steps[at] ?? value);
  return (
    <span className="armada-slider">
      <input
        {...rest}
        type="range"
        className="armada-slider__input"
        min={0}
        max={steps.length - 1}
        step={1}
        value={at}
        aria-valuetext={shown}
        onChange={(event) => {
          const next = steps[Number(event.target.value)];
          if (next !== undefined) onChange(next);
        }}
      />
      <span className="armada-slider__value" aria-hidden="true">
        {shown}
      </span>
    </span>
  );
}

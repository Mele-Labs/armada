import type { InputHTMLAttributes, ReactNode } from "react";

/**
 * One choice out of a set, and its label, as one hit target.
 *
 * Radio inherits the input's border and focus rules. Checked takes the accent
 * on both the ring and the dot — the accent is the affordance token, and a
 * chosen option is an interaction state rather than a status.
 */
export type RadioProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  /** Sentence case, no Wh- opener. */
  children: ReactNode;
};

export function Radio({ children, ...rest }: RadioProps) {
  return (
    <label className="armada-radio">
      <input {...rest} type="radio" className="armada-radio__input" />
      <span className="armada-radio__ring" aria-hidden="true">
        <span className="armada-radio__dot" />
      </span>
      <span>{children}</span>
    </label>
  );
}

export type RadioGroupProps = {
  /** Sentence case, no Wh- opener. Names what the set chooses between. */
  label?: string;
  /** The id of the text that already names the set, such as the row's own label. The set then draws no label of its own. */
  labelledBy?: string;
  children: ReactNode;
};

/**
 * The set a radio belongs to. It carries the group label and the stacking,
 * because one radio on its own is a choice with nothing to choose against.
 */
export function RadioGroup({ label, labelledBy, children }: RadioGroupProps) {
  return (
    <div className="armada-radio-group" role="radiogroup" aria-label={label} aria-labelledby={labelledBy}>
      {label !== undefined && <span className="armada-radio-group__label">{label}</span>}
      {children}
    </div>
  );
}

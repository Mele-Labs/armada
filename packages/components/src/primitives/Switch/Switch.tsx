import type { InputHTMLAttributes, ReactNode } from "react";

/**
 * A setting that is on or off, with its label on the left and the control on
 * the right.
 *
 * **The track and thumb are cut, not rounded**: two chamfered corners each, a grip ridged into the thumb, and `--accent` when
 * on. `data-switch="rail"` on the root draws the track as a rail the thumb rides on.
 *
 * The optional description states what the setting does when on and what
 * happens when off. It is not help text for the label — a label that needs
 * explaining is the wrong label.
 */
export type SwitchProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  /** Sentence case, no Wh- opener. */
  children: ReactNode;
  /** What happens when on, and what happens when off. */
  description?: ReactNode;
};

function Control(rest: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return (
    <>
      <input {...rest} type="checkbox" role="switch" className="armada-switch__input" />
      <span className="armada-switch__track" aria-hidden="true">
        <span className="armada-switch__thumb" />
      </span>
    </>
  );
}

export function Switch({ children, description, ...rest }: SwitchProps) {
  return (
    <label className="armada-switch" data-described={description !== undefined || undefined}>
      <span className="armada-switch__text">
        <span>{children}</span>
        {description !== undefined && (
          <span className="armada-switch__description">{description}</span>
        )}
      </span>
      <Control {...rest} />
    </label>
  );
}

/**
 * The switch alone, for a row whose text beside it does something else when pressed. `label` names it for a screen reader,
 * and only the control itself toggles.
 */
export function SwitchControl({ label, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "children"> & { label: string }) {
  return (
    <label className="armada-switch" data-control="">
      <Control {...rest} aria-label={label} />
    </label>
  );
}

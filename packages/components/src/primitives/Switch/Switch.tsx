import type { InputHTMLAttributes, ReactNode } from "react";

/**
 * A setting that is on or off, with its label on the left and the control on
 * the right.
 *
 * **A recessed pill with a raised knob, and the state written in the half the knob leaves**: ON on the lit track, OFF on the
 * dark one, or the two `words` given. The track widens to the longer word, so a pair like 12H and 24H reads as written.
 * `size="sm"` is a band's or a bar's switch, one badge tall; a row's is the default.
 *
 * The optional description states what the setting does when on and what
 * happens when off. It is not help text for the label — a label that needs
 * explaining is the wrong label.
 */
export type SwitchProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> &
  Look & {
    /** Sentence case, no Wh- opener. */
    children: ReactNode;
    /** What happens when on, and what happens when off. */
    description?: ReactNode;
  };

type Look = {
  /** What the track says in each state, a word or two short. ON and OFF where absent. */
  words?: { on: string; off: string };
  /** `sm` for a switch in a band or a bar. */
  size?: "sm";
};

function Control({ words = { on: "ON", off: "OFF" }, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "size"> & Pick<Look, "words">) {
  return (
    <>
      <input {...rest} type="checkbox" role="switch" className="armada-switch__input" />
      <span className="armada-switch__track" aria-hidden="true">
        <span className="armada-switch__word" data-word="on">
          {words.on}
        </span>
        <span className="armada-switch__word" data-word="off">
          {words.off}
        </span>
        <span className="armada-switch__thumb" />
      </span>
    </>
  );
}

export function Switch({ children, description, size, ...rest }: SwitchProps) {
  return (
    <label className="armada-switch" data-described={description !== undefined || undefined} data-size={size}>
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
export function SwitchControl({ label, size, ...rest }: Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "children" | "size"> & Look & { label: string }) {
  return (
    <label className="armada-switch" data-control="" data-size={size}>
      <Control {...rest} aria-label={label} />
    </label>
  );
}

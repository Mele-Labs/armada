import type { ButtonHTMLAttributes } from "react";

/**
 * A row's select control, drawn in Armada's own hand rather than as a form's box: a sight's four corners that close on a
 * lit square. One press, a `checkbox` to assistive tech, and it holds no state. `sm` is the same mark a size down, for a
 * row that stands under another.
 */
export type SelectMarkProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type" | "role"> & {
  checked: boolean;
  onToggle: () => void;
  /** The accessible name: `Select <the row>`. */
  label: string;
  size?: "sm";
};

export function SelectMark({ checked, onToggle, label, size, className, ...rest }: SelectMarkProps) {
  return (
    <button
      {...rest}
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={label}
      className={`armada-select-mark${className === undefined ? "" : ` ${className}`}`}
      data-size={size}
      data-checked={checked || undefined}
      onClick={onToggle}
    >
      <svg className="armada-select-mark__reticle" viewBox="0 0 24 24" aria-hidden="true">
        <path className="armada-select-mark__corners" d="M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5" />
        <rect className="armada-select-mark__core" x="8" y="8" width="8" height="8" rx="1" />
      </svg>
    </button>
  );
}

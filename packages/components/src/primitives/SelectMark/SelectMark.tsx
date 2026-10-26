import type { ButtonHTMLAttributes } from "react";

/**
 * A row's select control, drawn in Armada's own hand rather than as a form's box. **Two looks are offered while the
 * owner picks one**: `reticle`, a sight's four corners that close on a lit square, and `plate`, the row's place in its
 * list as a mono plate that fills with the accent. Either is one press, a `checkbox` to assistive tech, and holds no state.
 */
export type SelectMarkLook = "reticle" | "plate";

export type SelectMarkProps = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type" | "role"> & {
  checked: boolean;
  onToggle: () => void;
  /** The accessible name: `Select <the row>`. */
  label: string;
  look?: SelectMarkLook;
  /** The row's place in its list, from 1, which the plate shows. */
  index?: number;
};

export function SelectMark({ checked, onToggle, label, look = "reticle", index = 1, className, ...rest }: SelectMarkProps) {
  return (
    <button
      {...rest}
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={label}
      className={`armada-select-mark${className === undefined ? "" : ` ${className}`}`}
      data-look={look}
      data-checked={checked || undefined}
      onClick={onToggle}
    >
      {look === "plate" ? (
        <span className="armada-select-mark__plate" aria-hidden="true">
          {String(index).padStart(2, "0")}
        </span>
      ) : (
        <svg className="armada-select-mark__reticle" viewBox="0 0 24 24" aria-hidden="true">
          <path className="armada-select-mark__corners" d="M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5" />
          <rect className="armada-select-mark__core" x="8" y="8" width="8" height="8" rx="1" />
        </svg>
      )}
    </button>
  );
}

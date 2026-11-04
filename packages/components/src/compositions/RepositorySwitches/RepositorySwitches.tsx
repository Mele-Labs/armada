import type { ReactNode } from "react";

import { SwitchControl } from "../../primitives/Switch/Switch";

/** One repository Fleet serves, and whether the switch is on for it. */
export type RepositorySwitchRow = {
  id: string;
  name: string;
  on: boolean;
};

export type RepositorySwitchesProps = {
  /** The list's heading, and its name for a screen reader. */
  title: string;
  /** One line above the list: what the switch does. */
  help?: ReactNode;
  rows: readonly RepositorySwitchRow[];
  /** Each switch's name for a screen reader, from its repository's. */
  switchLabel: (name: string) => string;
  onSwitch: (id: string, on: boolean) => void;
  disabled?: boolean;
};

/**
 * One switch per repository, the same whichever repository the rail has picked. **The switch sits in a fixed column**,
 * so the switches stand in a line however long a name is.
 */
export function RepositorySwitches({
  title,
  help,
  rows,
  switchLabel,
  onSwitch,
  disabled,
}: RepositorySwitchesProps) {
  return (
    <section className="armada-repository-switches" aria-label={title}>
      <h4 className="armada-repository-switches__title" aria-hidden="true">
        {title}
      </h4>
      {help === undefined ? null : <p className="armada-repository-switches__help">{help}</p>}
      <ul className="armada-repository-switches__rows">
        {rows.map((one) => (
          <li key={one.id} className="armada-repository-switches__row">
            <span className="armada-repository-switches__name">{one.name}</span>
            <span className="armada-repository-switches__switch">
              <SwitchControl
                size="sm"
                label={switchLabel(one.name)}
                checked={one.on}
                {...(disabled === true
                  ? {
                      disabled: true,
                    }
                  : {})}
                onChange={(event) => onSwitch(one.id, event.target.checked)}
              />
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

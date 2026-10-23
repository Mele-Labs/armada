/**
 * What a taught mod holds, drawn under its row for any kind: a part for each thing it changes, and under it a fact for each
 * thing it says. A fact that was found rather than said carries where it came from. A fact with no label continues the one above.
 */
export type ModRulesProps = {
  /** The mod's name, which names the group. */
  name: string;
  parts: readonly {
    part: string;
    facts: readonly {
      /** Empty for a fact that continues the label above it, as the places a colour appears do. */
      label: string;
      value: string;
      /** The value is a pattern or an address, drawn in the mono face. */
      code?: boolean;
      /** Where the value was found. Absent where it was said. */
      from?: string;
    }[];
  }[];
};

export function ModRules({ name, parts }: ModRulesProps) {
  return (
    <div className="armada-mod-rules" role="group" aria-label={`Details of ${name}`}>
      {parts.map((one) => (
        <section key={one.part} className="armada-mod-rules__part" aria-label={one.part}>
          <h4 className="armada-mod-rules__head">{one.part}</h4>
          <dl className="armada-mod-rules__facts">
            {one.facts.map((fact) => (
              <div key={`${fact.label}:${fact.value}`} className="armada-mod-rules__fact">
                <dt aria-hidden={fact.label === "" || undefined}>{fact.label}</dt>
                <dd data-code={fact.code || undefined}>{fact.value}</dd>
                <dd className="armada-mod-rules__from">{fact.from}</dd>
              </div>
            ))}
          </dl>
        </section>
      ))}
    </div>
  );
}

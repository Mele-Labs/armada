// Autopilot's four settings as the start form holds them, each with the words Fleet read it from where the form is a reading.
// #2092, #2105, `docs/concepts/autopilot.md`.

import { Input } from "../../primitives/Input/Input";

/** The four settings as the start form holds them, each a field's text. */
export type AutopilotFields = { atOnce: string; until: string; spend: string; sessions: string };

/** What Fleet read the sentence as: the fields, and the person's own words each was read from, where it said any. */
export type AutopilotReading = { fields: AutopilotFields; quoted: Readonly<Partial<Record<keyof AutopilotFields, string>>> };

const FIELDS: readonly (readonly [keyof AutopilotFields, string, string])[] = [
  ["atOnce", "At once", "1"],
  ["until", "Until", "no time set"],
  ["spend", "Spend, in dollars", "no cap"],
  ["sessions", "Jobs", "no cap"],
];

/** The four settings as fields, each with the words it was read from where the form is a reading. */
function Fields({ fields, onChange, quoted }: { fields: AutopilotFields; onChange: (fields: AutopilotFields) => void; quoted?: AutopilotReading["quoted"] }) {
  return (
    <span className="armada-autopilot__fields">
      {FIELDS.map(([key, label, placeholder]) => (
        <span key={key} className="armada-autopilot__field">
          <Input label={label} mono value={fields[key]} placeholder={placeholder} onChange={(event) => onChange({ ...fields, [key]: event.currentTarget.value })} />
          {quoted === undefined ? null : <span className="armada-autopilot__said">{quoted[key] === undefined ? "not said" : `“${quoted[key]}”`}</span>}
        </span>
      ))}
    </span>
  );
}

export { Fields as AutopilotFieldInputs };

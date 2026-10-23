/**
 * What else a taught entry changed, drawn under its text on the Logbook page: one line for each of Bridge and Fleet that
 * the teaching reached. **Forget on the entry takes all of it back**, which is why it is listed here and nowhere else.
 */
export type AlsoChangesProps = {
  lines: readonly { part: "Bridge" | "Fleet"; text: string }[];
};

export function AlsoChanges({ lines }: AlsoChangesProps) {
  if (lines.length === 0) return null;
  return (
    <div className="armada-also-changes" role="group" aria-label="Also changes">
      <h3 className="armada-also-changes__head">Also changes</h3>
      <ul className="armada-also-changes__lines">
        {lines.map((line) => (
          <li key={line.part}>
            <span className="armada-also-changes__part">{line.part}</span>
            <span>{line.text}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

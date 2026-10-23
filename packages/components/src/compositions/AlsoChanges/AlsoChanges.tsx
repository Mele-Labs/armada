import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What else a taught entry changed, drawn under its text on the Logbook page: one line for each of Bridge and Fleet that
 * the teaching reached, and the rules mod that keeps it. **Forget on the entry takes all of it back**, including the mod,
 * which is why it is listed here and nowhere else.
 */
export type AlsoChangesProps = {
  lines: readonly {
    part: "Bridge" | "Fleet";
    text: string;
    /** The rules mod that keeps this change, named, and a press that opens it in Mods. */
    mod?: { name: string; onOpen: () => void };
  }[];
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
            {line.mod === undefined ? null : (
              <Tooltip label="Open in Mods">
                <Button variant="ghost" size="sm" className="armada-also-changes__mod" aria-label={`Open ${line.mod.name} in Mods`} onClick={line.mod.onOpen}>
                  {line.mod.name}
                </Button>
              </Tooltip>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

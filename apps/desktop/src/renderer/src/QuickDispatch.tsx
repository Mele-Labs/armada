// The Dashboard's dispatch bar: one slim line over every tab. It takes focus where nothing needs the
// owner; the first words typed hand over to the composer, which opens holding them and grows in
// (`.armada-screen__pane--grown`). Nothing is sent from here.

import { CornerDownLeft } from "lucide-react";
import { REQUEST_PLACEHOLDER } from "@armada/components";

export function QuickDispatch({ onType, focused }: { onType: (words: string) => void; focused: boolean }) {
  return (
    <label className="armada-dispatch-bar">
      <span className="armada-dispatch-bar__prompt" aria-hidden="true">
        <CornerDownLeft size={20} />
      </span>
      <input
        aria-label="Request"
        className="armada-dispatch-bar__field"
        autoFocus={focused}
        value=""
        placeholder={REQUEST_PLACEHOLDER}
        onChange={(event) => event.target.value !== "" && onType(event.target.value)}
      />
    </label>
  );
}

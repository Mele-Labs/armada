// System › Thread: how a Job's thread reads and the quick bar's lines, the same form the Aa panel in a Job's head draws, with
// Voice under it. Its Spacing is the thread's own; System › Spacing sets the rest of Bridge.

import { useState } from "react";
import { DEFAULT_LOOK, ThreadSettings, useThreadLook } from "@armada/components";
import type { ThreadLook } from "@armada/components";

const NO_MODEL = { state: "absent" } as const;

export function ThreadPane() {
  const source = useThreadLook();
  // A window that holds no look still draws the form; what it sets is the pane's own until something reads it.
  const [own, setOwn] = useState<ThreadLook>(DEFAULT_LOOK);
  return (
    <ThreadSettings
      look={source?.look ?? own}
      onChange={source?.setLook ?? setOwn}
      voice={{ whisper: source?.whisper ?? NO_MODEL, onInstall: source?.installWhisper ?? (() => undefined) }}
      keep="thread-look:new-line:system"
    />
  );
}

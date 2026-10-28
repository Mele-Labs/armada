// Open work in one place: the pull requests, issues and follow-ups still to settle, drawn after the last step so a folded step never hides them. A right press on the
// group hides it for every Job; Settings, Bridge has it back.

import { useState, type ReactNode } from "react";
import { EyeOff } from "lucide-react";
import { DropdownMenu } from "@armada/components";

import { saveSetting, useSetting } from "../use-setting";

const KEY = "bridge.openGroup";

/** Whether the Open group is drawn: on until a person hides it. */
export const useOpenGroupShown = (): boolean => useSetting(KEY) !== false;

/** The group's panel with its hide menu, at the pointer on a right press. */
export function OpenGroupMenu({ children }: { children: ReactNode }) {
  const [at, setAt] = useState<{ x: number; y: number }>();
  return (
    <div style={{ display: "contents" }} onContextMenu={(event) => (event.preventDefault(), event.stopPropagation(), setAt({ x: event.clientX, y: event.clientY }))}>
      {children}
      {at === undefined ? null : (
        <DropdownMenu
          triggerLabel="Open group"
          at={at}
          open
          entries={[{ kind: "item", id: "hide", label: "Hide Open group", icon: EyeOff }]}
          onOpenChange={(open) => open || setAt(undefined)}
          onSelect={() => void saveSetting(KEY, false)}
        />
      )}
    </div>
  );
}

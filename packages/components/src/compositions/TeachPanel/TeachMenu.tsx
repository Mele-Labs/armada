import { useState } from "react";
import type { KeyboardEvent, MouseEvent, ReactNode } from "react";
import { GraduationCap } from "lucide-react";

import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";

type At = { x: number; y: number; from: HTMLElement };

/** What a row spreads onto its own element, and the menu to draw beside it. */
export type TeachMenu = {
  on: { onContextMenu?: (event: MouseEvent<HTMLElement>) => void; onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void };
  menu: ReactNode;
};

/**
 * The menu a row offers Teach in: a right press opens it at the pointer, and the ContextMenu key or Shift+F10 under the row.
 * `heading` names what the row is. Absent `onTeach`, the row has no menu and spreads nothing.
 */
export function useTeachMenu(heading: string, onTeach: (() => void) | undefined): TeachMenu {
  const [at, setAt] = useState<At | undefined>(undefined);
  if (onTeach === undefined) return { on: {}, menu: null };
  const on: TeachMenu["on"] = {
    onContextMenu: (event) => {
      // A row inside this one has opened its own menu; the outer one stays shut.
      if (event.defaultPrevented) return;
      event.preventDefault();
      setAt({ x: event.clientX, y: event.clientY, from: event.currentTarget });
    },
    onKeyDown: (event) => {
      if (event.defaultPrevented || (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey))) return;
      event.preventDefault();
      const box = event.currentTarget.getBoundingClientRect();
      setAt({ x: box.left + box.width / 2, y: box.bottom, from: event.currentTarget });
    },
  };
  const menu =
    at === undefined ? null : (
      <DropdownMenu
        triggerLabel={heading}
        at={{ x: at.x, y: at.y }}
        open
        entries={[
          { kind: "label", id: "heading", label: heading },
          { kind: "item", id: "teach", label: "Teach", icon: GraduationCap },
        ]}
        onOpenChange={(open) => {
          if (open) return;
          setAt(undefined);
          at.from.focus();
        }}
        onSelect={(chosen) => {
          if (chosen === "teach") onTeach();
        }}
      />
    );
  return { on, menu };
}

// What a send leaves over the one bar's field: one slim strip in the cockpit's band style, holding the kind's
// mark, the Autopilot's repository, and the way to open it. It stays until the next dispatch or a dismiss
// and shows only the latest. A Session opens itself and leaves none.

import { X } from "lucide-react";
import type { CSSProperties, KeyboardEvent, Ref } from "react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { KIND_ICON, KIND_LABEL } from "./kinds";
import "./dispatch-strip.css";

/** The latest send, as the strip draws it. */
export type SentView = {
  /** Which send this is, so one after another is told from one still standing. */
  id: number;
  kind: "autopilot";
  /** The Autopilot's repository. */
  title: string;
};

export function DispatchedStrip({
  sent,
  onOpen,
  onDismiss,
  onKeyDown,
  rootRef,
  style,
}: {
  sent: SentView;
  onOpen: () => void;
  onDismiss: () => void;
  /** The keys of an entry whose focus is on the strip itself. */
  onKeyDown?: (event: KeyboardEvent<HTMLElement>) => void;
  /** Where the cursor lands when the entry has none of its own. */
  rootRef?: Ref<HTMLDivElement>;
  style?: CSSProperties;
}) {
  const Mark = KIND_ICON[sent.kind];
  return (
    <div
      ref={rootRef}
      className="armada-dispatch-strip"
      role="group"
      aria-label="Last dispatch"
      data-kind={sent.kind}
      data-openable=""
      tabIndex={-1}
      style={style}
      onClick={onOpen}
      onKeyDown={onKeyDown}
    >
      <Tooltip label={KIND_LABEL[sent.kind]}>
        <span className="armada-dispatch-strip__mark">
          <Mark size={16} aria-hidden />
        </span>
      </Tooltip>
      <span className="armada-dispatch-strip__title">{sent.title}</span>
      <Kbd className="armada-dispatch-strip__enter">↵</Kbd>
      <Button
        variant="secondary"
        size="sm"
        onClick={(event) => {
          event.stopPropagation();
          onOpen();
        }}
      >
        Open
      </Button>
      <Tooltip label="Dismiss">
        <Button
          variant="ghost"
          size="sm"
          iconOnly
          aria-label="Dismiss"
          onClick={(event) => {
            event.stopPropagation();
            onDismiss();
          }}
        >
          <X size={14} aria-hidden />
        </Button>
      </Tooltip>
    </div>
  );
}

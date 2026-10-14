// What a send leaves over the one bar's field: one slim strip in the cockpit's band style, holding the kind's
// mark, the Job's title or the Autopilot's repository, and the way to open it. It stays until the next
// dispatch or a dismiss and shows only the latest. A Session opens itself and leaves none. Before the
// Job's id is known the mark is a turning ring; a refusal puts Fleet's words where the title was.

import { LoaderCircle, X } from "lucide-react";
import type { CSSProperties, KeyboardEvent, Ref } from "react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { KIND_ICON, KIND_LABEL } from "./kinds";
import "./dispatch-strip.css";

/** The latest send, as the strip draws it. */
export type SentView = {
  /** Which send this is, so one after another is told from one still standing. */
  id: number;
  kind: "job" | "autopilot";
  /** A Job's request, first line; an Autopilot's repository. */
  title: string;
  /** What it opens is known. A Job's is not until Fleet has made its row. */
  ready: boolean;
  /** Fleet's refusal words. */
  refused?: string;
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
  const pending = !sent.ready && sent.refused === undefined;
  const openable = sent.ready && sent.refused === undefined;
  return (
    <div
      ref={rootRef}
      className="armada-dispatch-strip"
      role="group"
      aria-label="Last dispatch"
      data-kind={sent.kind}
      data-refused={sent.refused === undefined ? undefined : ""}
      data-openable={openable || undefined}
      tabIndex={-1}
      style={style}
      onClick={() => openable && onOpen()}
      onKeyDown={onKeyDown}
    >
      {pending ? (
        <Tooltip label="Dispatching">
          <span className="armada-dispatch-strip__mark" data-turning>
            <LoaderCircle size={16} aria-hidden />
          </span>
        </Tooltip>
      ) : (
        <Tooltip label={KIND_LABEL[sent.kind]}>
          <span className="armada-dispatch-strip__mark">
            <Mark size={16} aria-hidden />
          </span>
        </Tooltip>
      )}
      <span className="armada-dispatch-strip__title">{sent.refused ?? sent.title}</span>
      {openable ? (
        <>
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
        </>
      ) : null}
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

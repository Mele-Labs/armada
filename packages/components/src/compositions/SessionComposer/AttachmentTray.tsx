import { Trash2 } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { ComposerFile } from "./SessionComposer";

/** What a file is called on its tile: the part after the last dot, or nothing for a name with none. */
const extensionOf = (name: string): string | null => {
  const dot = name.lastIndexOf(".");
  return dot <= 0 || dot === name.length - 1 ? null : name.slice(dot + 1).toUpperCase();
};

/**
 * What waits to go with the message in the spacious take, over the words: a picture as itself, a file or a
 * sketch as a tile with its name. Each takes off with its own Remove, reached by Tab.
 */
export function AttachmentTray({
  files,
  drawn,
  onRemoveFile,
  onRemoveDrawn,
}: {
  files: readonly ComposerFile[];
  drawn: readonly {
    id: string;
    title: string;
  }[];
  onRemoveFile: (id: string) => void;
  onRemoveDrawn: (id: string) => void;
}) {
  return (
    <div className="armada-tray" role="group" aria-label="Attached">
      {files.map((one) =>
        one.src === undefined ? (
          <Tile
            key={one.id}
            name={one.name}
            kind={extensionOf(one.name)}
            onRemove={() => onRemoveFile(one.id)}
          />
        ) : (
          <Tooltip key={one.id} label={one.name}>
            <span className="armada-tray__tile" data-picture="">
              <img className="armada-tray__picture" src={one.src} alt={one.name} />
              <Remove name={one.name} onRemove={() => onRemoveFile(one.id)} />
            </span>
          </Tooltip>
        ),
      )}
      {drawn.map((one) => (
        <Tile key={one.id} name={one.title} kind="Sketch" onRemove={() => onRemoveDrawn(one.id)} />
      ))}
    </div>
  );
}

function Tile({
  name,
  kind,
  onRemove,
}: {
  name: string;
  kind: string | null;
  onRemove: () => void;
}) {
  return (
    <span className="armada-tray__tile">
      <span className="armada-tray__name">{name}</span>
      {kind === null ? null : <span className="armada-tray__kind">{kind}</span>}
      <Remove name={name} onRemove={onRemove} />
    </span>
  );
}

function Remove({ name, onRemove }: { name: string; onRemove: () => void }) {
  return (
    <button
      type="button"
      className="armada-tray__remove"
      aria-label={`Remove ${name}`}
      onClick={onRemove}
    >
      <Trash2 size={12} strokeWidth={2} aria-hidden />
    </button>
  );
}

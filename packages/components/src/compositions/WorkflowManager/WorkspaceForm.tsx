// Making, renaming and removing a Workspace (armada-cwj, mock takes): its name and the repositories in it. Presentational: the host holds the Workspaces and answers each press.

import { Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Repository } from "../LaneCanvas/model";

/** Where the form is drawn: a centred sheet, the app's side panel, or a layer under the button. The `sheet` take is the only one that draws its own layer. */
export type WorkspaceTake = "sheet" | "panel" | "popover";

/** Where a Workspace is kept: on this Mac, or committed in the repository, where only a pull request removes it. */
export type WorkspaceKept = "mac" | "repository";

export type WorkspacesEditing = {
  take: WorkspaceTake;
  /** Every repository Fleet serves. */
  repositories: readonly Repository[];
  /** The repository the page is on, which a Workspace made here holds. */
  current: string;
  /** Where the Workspace on show is kept. Absent where the repository is in none. */
  kept?: WorkspaceKept;
  onCreate: (name: string, repositories: readonly string[]) => void;
  onSave: (name: string, repositories: readonly string[]) => void;
  onRemove: () => void;
};

export type WorkspaceFormProps = {
  /** `dialog` draws the centred sheet with its own Create and Cancel. `inline` is a form that fills the place it is given. */
  layout: "dialog" | "inline";
  mode: "new" | "edit";
  repositories: readonly Repository[];
  current: string;
  /** The Workspace's name and the repositories in it, when editing. */
  name?: string;
  ticked?: readonly string[];
  /** Where it is kept, when editing: a committed one is not removed here. */
  kept?: WorkspaceKept;
  onSubmit: (name: string, repositories: readonly string[]) => void;
  onRemove?: () => void;
  onClose: () => void;
};

const LIVES_IN_REPOSITORY = "Lives in the repository";

export function WorkspaceForm({ layout, mode, repositories, current, name = "", ticked, kept, onSubmit, onRemove, onClose }: WorkspaceFormProps) {
  const field = useRef<HTMLInputElement>(null);
  const [text, setText] = useState(name);
  const [chosen, setChosen] = useState<readonly string[]>(ticked ?? [current]);
  const trimmed = text.trim();
  const ready = trimmed !== "" && chosen.length > 0;
  const submit = () => {
    if (!ready) return;
    onSubmit(trimmed, chosen);
    onClose();
  };
  const remove = () => {
    onRemove?.();
    onClose();
  };
  // Focus lands on the name once the layer has taken its own focus.
  useEffect(() => {
    const timer = window.setTimeout(() => field.current?.focus(), 0);
    return () => window.clearTimeout(timer);
  }, []);

  const fields = (
    <div className="armada-wm-ws">
      <Input ref={field} label="Name" value={text} onChange={(event) => setText(event.target.value)} />
      <fieldset className="armada-wm-ws__repos">
        <legend>Repositories</legend>
        {repositories.map((one) => (
          <Checkbox
            key={one.id}
            checked={chosen.includes(one.id)}
            // The repository the page is on stays in: the tabs under it are drawn for it.
            disabled={one.id === current}
            onChange={(event) => setChosen((was) => (event.target.checked ? [...was, one.id] : was.filter((each) => each !== one.id)))}
          >
            {one.name}
          </Checkbox>
        ))}
      </fieldset>
      {mode === "edit" && onRemove !== undefined ? (
        <div className="armada-wm-ws__remove">
          {kept === "repository" ? (
            <Tooltip label={LIVES_IN_REPOSITORY}>
              <Button type="button" variant="ghost" size="sm" disabled>
                <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
                Remove Workspace
              </Button>
            </Tooltip>
          ) : (
            <Button type="button" variant="ghost" size="sm" onClick={remove}>
              <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
              Remove Workspace
            </Button>
          )}
        </div>
      ) : null}
    </div>
  );

  if (layout === "dialog") {
    return (
      <Dialog open tone="neutral" title={mode === "new" ? "New Workspace" : `Edit ${name}`} confirmLabel={mode === "new" ? "Create" : "Save"} confirmDisabled={!ready} onConfirm={submit} onCancel={onClose}>
        {fields}
      </Dialog>
    );
  }
  const keys = (event: KeyboardEvent<HTMLFormElement>) => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    onClose();
  };
  return (
    <form
      className="armada-wm-ws__form"
      aria-label={mode === "new" ? "New Workspace" : `Edit ${name}`}
      onKeyDown={keys}
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      {fields}
      <div className="armada-wm-ws__actions">
        <Button type="button" variant="ghost" size="sm" onClick={onClose}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" size="sm" disabled={!ready}>
          {mode === "new" ? "Create" : "Save"}
        </Button>
      </div>
    </form>
  );
}

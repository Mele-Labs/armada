// A Workspace's name and the repositories in it (armada-cwj). Presentational: the host holds the Workspaces and answers each press.

import { Trash2 } from "lucide-react";
import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Repository } from "../LaneCanvas/model";

export type WorkspaceFormProps = {
  mode: "new" | "edit";
  /** Every repository Fleet serves. */
  repositories: readonly Repository[];
  /** The Workspace's name and the repositories in it by id, when editing. */
  name?: string;
  ticked?: readonly string[];
  /** The repository a committed Workspace lives in: its Remove is off, and says so. Absent where it is kept on this Mac. */
  home?: string;
  onSubmit: (name: string, repositories: readonly string[]) => void;
  onRemove?: () => void;
};

export function WorkspaceForm({ mode, repositories, name = "", ticked = [], home, onSubmit, onRemove }: WorkspaceFormProps) {
  const [text, setText] = useState(name);
  const [chosen, setChosen] = useState<readonly string[]>(ticked);
  const trimmed = text.trim();
  const ready = trimmed !== "" && chosen.length > 0;
  const remove = (
    <Button type="button" variant="ghost" size="sm" disabled={home !== undefined} onClick={onRemove}>
      <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
      Remove
    </Button>
  );
  return (
    <form
      className="armada-wm-ws__form"
      aria-label={mode === "new" ? "New Workspace" : `Edit ${name}`}
      onSubmit={(event) => {
        event.preventDefault();
        if (ready) onSubmit(trimmed, chosen);
      }}
    >
      <Input label="Name" value={text} onChange={(event) => setText(event.target.value)} />
      <fieldset className="armada-wm-ws__repos">
        <legend>Repositories</legend>
        {repositories.map((one) => (
          <Checkbox
            key={one.id}
            checked={chosen.includes(one.id)}
            onChange={(event) => setChosen((was) => (event.target.checked ? [...was, one.id] : was.filter((each) => each !== one.id)))}
          >
            {one.name}
          </Checkbox>
        ))}
      </fieldset>
      <div className="armada-wm-ws__actions">
        {mode === "new" || onRemove === undefined ? null : home === undefined ? remove : <Tooltip label={`Lives in ${home}`}>{remove}</Tooltip>}
        <Button type="submit" variant="primary" size="sm" disabled={!ready}>
          {mode === "new" ? "Create" : "Save"}
        </Button>
      </div>
    </form>
  );
}

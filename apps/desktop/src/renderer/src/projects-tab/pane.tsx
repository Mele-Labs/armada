// The main pane, for whatever is open: a view, a repository or a project. Its head holds the title, Dispatch (drawn only once cards
// are picked), the toggle that shows done tasks and the key that goes to the cards; under it the field that adds a task, then the
// cards. A view or a repository is cut into groups by project under small quiet heads, the tasks in no project first; a project is
// one list in his order, moved by a drag and ⌥↑ ⌥↓. A card names its repository only where the list holds more than one.

import { ListChecks } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import type { Band } from "./list";
import { PROJECTS_KEYS } from "./key-table";
import type { ProjectRow } from "./model";
import { AddTask, chosenOf, useCards, type AddScope, type Hosts } from "./project";
import { isDone, useProjects, useProjectsStore } from "./store";
import { repositoriesIn, type Group } from "./views";

type Props = Omit<Hosts, "onDispatch"> & {
  /** The pane's name. */
  label: string;
  title: string;
  groups: readonly Group[];
  bands: readonly Band[];
  /** Where the add field writes, or none where a task added would not show. */
  scope: AddScope | undefined;
  /** The cards picked, to dispatch: the pane splits them by repository. */
  onDispatch: (rows: readonly ProjectRow[]) => void;
};

export function Pane({ label, title, groups, bands, scope, onDispatch, onOpenSession, onOpenLink }: Props) {
  const store = useProjectsStore()!;
  const { loose, projects, selected, showDone } = useProjects();
  const labels = new Map(bands.map((band) => [band.manifestId, band.label]));
  const { item, dragging } = useCards({ onOpenSession, onOpenLink, onDispatch }, labels);
  // The picks stand in any list; a done card hidden is not one.
  const picks = [loose, ...projects.map((one) => one.rows)].flatMap((rows) => chosenOf(rows, selected)).filter((row) => showDone || !isDone(row));
  const many = repositoriesIn(groups).length > 1;
  return (
    <section className="deck-pane projects-pane" aria-label={label} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{title}</h2>
        <span className="deck-pane__acts">
          {picks.length === 0 ? null : (
            <Tooltip label="Dispatch the cards picked" shortcut={PROJECTS_KEYS.dispatch}>
              <Button variant="secondary" size="sm" data-dispatch onClick={() => onDispatch(picks)}>
                Dispatch
              </Button>
            </Tooltip>
          )}
          <Tooltip label={showDone ? "Hide done" : "Show done"} shortcut={PROJECTS_KEYS.done} asChild>
            <button type="button" className="projects-head__act" data-done-toggle data-on={showDone || undefined} aria-label={showDone ? "Hide done" : "Show done"} onClick={() => store.showDone(!showDone)}>
              <ListChecks size={16} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
          <Tooltip label="Go to the tasks" shortcut={PROJECTS_KEYS.rows}>
            <Kbd>{PROJECTS_KEYS.rows}</Kbd>
          </Tooltip>
        </span>
      </header>
      {scope === undefined ? null : (
        <div className="projects-top">
          <AddTask bands={bands} scope={scope} />
        </div>
      )}
      <div className="projects-groups" data-reordering={dragging === undefined ? undefined : ""}>
        {groups.map((group) => {
          const rows = (
            <ul className="projects-rows" aria-label={group.title === undefined ? "Rows" : `Rows of ${group.title}`}>
              {group.rows.map((row) => item(row, group.key, { movable: group.movable, many }))}
            </ul>
          );
          return group.title === undefined ? (
            <div key={group.key} className="projects-group">
              {rows}
            </div>
          ) : (
            <section key={group.key} className="projects-group" aria-label={group.title} data-group={group.key}>
              <h3 className="projects-group__title">{group.title}</h3>
              {rows}
            </section>
          );
        })}
      </div>
    </section>
  );
}

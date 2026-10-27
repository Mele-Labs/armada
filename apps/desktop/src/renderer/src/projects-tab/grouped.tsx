// A list of tasks across projects, cut into groups with a head each: the pane both layout takes draw for a view, and the one
// the second take draws for everything. A project's group head renames and deletes it; a group's foot holds the field that
// writes a task into it. The cards are the project pane's.

import type { ReactNode } from "react";
import { Pencil, Trash2 } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";

import { RenameProject, type Band } from "./list";
import { PROJECTS_KEYS } from "./key-table";
import { AddTask, chosenIn, useCards, type Hosts } from "./project";
import { useProjects, useProjectsStore } from "./store";
import { fromOf, type Entry, type Group } from "./views";

type Props = Hosts & {
  /** The pane's name. */
  label: string;
  /** The head's left: a title, or a toolbar. */
  head: ReactNode;
  /** Under the head, before the groups: a name field being written. */
  below?: ReactNode;
  groups: readonly Group[];
  bands: readonly Band[];
  /** A field at each group's foot that has a project to write into. */
  adds: boolean;
  /** Heads of a project's group rename and delete it. */
  acts: boolean;
  onDispatchPicks: (picks: Entry[]) => void;
};

export function GroupedPane({ label, head, below, groups, bands, adds, acts, onDispatchPicks, ...hosts }: Props) {
  const store = useProjectsStore()!;
  const { projects, selected, renaming } = useProjects();
  const { item, dragging } = useCards(hosts);
  const picks = projects.flatMap((project) => chosenIn(project, selected).map((row) => ({ project, row })));
  return (
    <section className="deck-pane projects-pane projects-grouped" aria-label={label} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        {head}
        <span className="deck-pane__acts">
          <Tooltip label="Go to the tasks" shortcut={PROJECTS_KEYS.rows}>
            <Kbd>{PROJECTS_KEYS.rows}</Kbd>
          </Tooltip>
          <Tooltip label="Dispatch the rows selected to one Job" shortcut={PROJECTS_KEYS.dispatch}>
            <Button variant="secondary" size="sm" data-dispatch disabled={picks.length === 0} onClick={() => onDispatchPicks(picks)}>
              Dispatch
            </Button>
          </Tooltip>
        </span>
      </header>
      {below}
      <div className="projects-groups" data-reordering={dragging === undefined ? undefined : ""}>
        {groups.map((group) => (
          <section key={group.key} className="projects-group" aria-label={group.title} data-group={group.key}>
            <header className="projects-group__head" tabIndex={-1} data-project={group.project?.id}>
              {group.project !== undefined && renaming === group.project.id ? (
                <RenameProject id={group.project.id} name={group.project.name} />
              ) : (
                <h3 className="projects-group__title">{group.title}</h3>
              )}
              {acts && group.project !== undefined && renaming !== group.project.id ? (
                <span className="projects-group__acts">
                  <Tooltip label="Rename the project" shortcut={PROJECTS_KEYS.rename} asChild>
                    <button type="button" className="projects-row__act" tabIndex={-1} aria-label={`Rename ${group.project.name}`} onClick={() => store.renaming(group.project!.id)}>
                      <Pencil size={16} strokeWidth={2} aria-hidden />
                    </button>
                  </Tooltip>
                  <Tooltip label="Delete the project" shortcut={PROJECTS_KEYS.remove} asChild>
                    <button type="button" className="projects-row__act" tabIndex={-1} data-remove-project aria-label={`Delete ${group.project.name}`} onClick={() => store.removeProject(group.project!.id)}>
                      <Trash2 size={16} strokeWidth={2} aria-hidden />
                    </button>
                  </Tooltip>
                </span>
              ) : null}
            </header>
            {group.entries.length === 0 ? null : (
              <ul className="projects-rows projects-group__rows" aria-label={`Rows of ${group.title}`}>
                {group.entries.map((entry) => item(entry.project, entry.row, entry.project.id, undefined, fromOf(entry, bands, group.labelled)))}
              </ul>
            )}
            {adds && group.adds !== undefined ? (
              <div className="projects-group__add">
                <AddTask project={group.adds} />
              </div>
            ) : null}
          </section>
        ))}
      </div>
    </section>
  );
}

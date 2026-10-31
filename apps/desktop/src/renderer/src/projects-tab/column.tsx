// The left column: a search over every task, then three views of every task (all, the ones Fleet asks about, the ones a Job holds), a hairline, his projects as
// name-only rows with a `+` for a new one, a hairline, and the repositories. A project is his and spans repositories, so a new one
// asks for none. A repository is a view, all the tasks in it, and is not a list added to. Glyphs tell the two apart: a folder for a
// project, a branch for a repository. Selected is told by shape (`--select-ground`, `--select-frame`), never a band. While a
// search is active nothing in the column is selected: the search is what the pane shows.

import { useState, type KeyboardEvent, type MouseEvent } from "react";
import { Folder, GitBranch, Pencil, Plus, Trash2 } from "lucide-react";
import { DropdownMenu, SearchField, Tooltip } from "@armada/components";

import { ConfirmCard } from "./confirm-card";
import { NameProject, RenameProject, type Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import { useProjects, useProjectsStore } from "./store";
import { VIEW_IDS, VIEW_MARKS, type View } from "./views";

type Props = {
  bands: readonly Band[];
  view: View;
  /** The repository open as a view, by Manifest id. */
  repo: string | undefined;
  /** The words in the search field, and whether they make a search. */
  query: string;
  searching: boolean;
  onQuery: (words: string) => void;
  onView: (view: View) => void;
  onRepo: (manifestId: string) => void;
  onProject: (id: string) => void;
};

export function Column({ bands, view, repo, query, searching, onQuery, onView, onRepo, onProject }: Props) {
  const store = useProjectsStore()!;
  const [menu, setMenu] = useState<{ id: string; x: number; y: number } | undefined>(undefined);
  const openMenu = (id: string) => (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    setMenu({ id, x: event.clientX, y: event.clientY });
  };
  const openMenuByKey = (id: string) => (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
    event.preventDefault();
    const box = event.currentTarget.getBoundingClientRect();
    setMenu({ id, x: box.left + box.width / 2, y: box.bottom });
  };
  const [confirming, setConfirming] = useState<{ id: string; target: { left: number; top: number; width: number; height: number } } | undefined>(undefined);
  const removeFromMenu = (id: string) => {
    const items = [...document.querySelectorAll<HTMLElement>(".projects-item[data-project]")];
    const at = items.findIndex((item) => item.dataset.project === id);
    const next = items[at + 1] ?? items[at - 1];
    store.removeProject(id);
    next?.focus();
  };
  const snap = useProjects();
  return (
    <section className="deck-panel deck-list projects-list" aria-label="Projects" tabIndex={-1}>
      <ul className="deck-list__rows projects-list__rows">
        <li>
          <SearchField label="Search the tasks" value={query} onValue={onQuery} cap={PROJECTS_KEYS.search} />
        </li>
        <li>
          <ul className="deck-list__group projects-views" aria-label="Views">
            {VIEW_IDS.map((id, at) => {
              const { Glyph, said } = VIEW_MARKS[id];
              const here = !searching && snap.open === undefined && repo === undefined && view === id;
              return (
                <li key={id} className="deck-list__item">
                  <Tooltip label={said} shortcut={PROJECTS_KEYS.view(at)} asChild>
                    <button type="button" className="projects-item projects-view" data-view={id} data-selected={here || undefined} aria-current={here || undefined} onClick={() => onView(id)}>
                      <Glyph size={16} strokeWidth={2} aria-hidden />
                      {said}
                    </button>
                  </Tooltip>
                </li>
              );
            })}
          </ul>
        </li>
        <li className="projects-rule" role="separator" />
        <li className="projects-section" aria-label="Your projects">
          <div className="projects-section__head">
            <h3 className="projects-section__label">Projects</h3>
            <Tooltip label="New project" shortcut={capOf(PROJECTS_KEYS.newProject)} asChild>
              <button type="button" className="projects-band__new" aria-label="New project" onClick={() => store.name(true)}>
                <Plus size={16} strokeWidth={2} aria-hidden />
              </button>
            </Tooltip>
          </div>
          <ul className="deck-list__group">
            {snap.projects.map((one) => (
              <li key={one.id} className="deck-list__item">
                {snap.renaming === one.id ? (
                  <RenameProject id={one.id} name={one.name} />
                ) : (
                  <button type="button" className="projects-item" data-project={one.id} data-selected={(!searching && snap.open === one.id) || undefined} aria-current={(!searching && snap.open === one.id) || undefined} onClick={() => onProject(one.id)} onContextMenu={openMenu(one.id)} onKeyDown={openMenuByKey(one.id)}>
                    <Folder size={16} strokeWidth={2} aria-hidden />
                    <span className="projects-item__name">{one.name}</span>
                  </button>
                )}
              </li>
            ))}
          </ul>
          {snap.naming ? <NameProject /> : null}
          {confirming === undefined ? null : (
            <ConfirmCard
              target={confirming.target}
              title={`Delete ${snap.projects.find((one) => one.id === confirming.id)?.name ?? "project"}?`}
              confirmLabel="Delete"
              onConfirm={() => {
                removeFromMenu(confirming.id);
                setConfirming(undefined);
              }}
              onCancel={() => {
                setConfirming(undefined);
                document.querySelector<HTMLElement>(`.projects-item[data-project="${CSS.escape(confirming.id)}"]`)?.focus();
              }}
            >
              Its tasks go with it.
            </ConfirmCard>
          )}
          {menu === undefined ? null : (
            <DropdownMenu
              triggerLabel="Project"
              at={{ x: menu.x, y: menu.y }}
              open
              entries={[
                { kind: "item", id: "rename", label: "Rename", icon: Pencil, shortcut: capOf(PROJECTS_KEYS.rename) },
                { kind: "item", id: "delete", label: "Delete", icon: Trash2, danger: true, shortcut: capOf(PROJECTS_KEYS.remove) },
              ]}
              onOpenChange={(open) => {
                if (!open) setMenu(undefined);
              }}
              onSelect={(chosen) => {
                if (chosen === "rename") store.renaming(menu.id);
                if (chosen === "delete") {
                  const box = document.querySelector<HTMLElement>(`.projects-item[data-project="${CSS.escape(menu.id)}"]`)?.getBoundingClientRect();
                  if (box !== undefined) setConfirming({ id: menu.id, target: { left: box.left, top: box.top, width: box.width, height: box.height } });
                }
              }}
            />
          )}
        </li>
        <li className="projects-rule" role="separator" />
        <li className="projects-section" aria-label="Repositories">
          <div className="projects-section__head">
            <h3 className="projects-section__label">Repositories</h3>
          </div>
          <ul className="deck-list__group">
            {bands.map((band) => {
              const here = !searching && snap.open === undefined && repo === band.manifestId;
              return (
                <li key={band.manifestId} className="deck-list__item">
                  <button type="button" className="projects-item" data-repo={band.manifestId} data-selected={here || undefined} aria-current={here || undefined} onClick={() => onRepo(band.manifestId)}>
                    <GitBranch size={16} strokeWidth={2} aria-hidden />
                    <span className="projects-item__name">{band.label}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        </li>
      </ul>
    </section>
  );
}

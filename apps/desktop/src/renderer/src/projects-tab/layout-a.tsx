// Take A of the Projects tab, views on top and repositories that fold. The left column holds three views of every task across
// repositories, a hairline, then a quiet row for each repository: it opens to its own tasks and its projects as name-only rows,
// with a `+` for a new project that is always drawn.

import { ChevronDown, ChevronRight, Plus } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";

import { NameProject, RenameProject, type Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import { ownId } from "./model";
import { useProjects, useProjectsStore } from "./store";
import { VIEW_IDS, VIEW_MARKS, type View } from "./views";

type Props = {
  bands: readonly Band[];
  view: View;
  onView: (view: View) => void;
  /** The repositories folded shut, by Manifest id. */
  folded: ReadonlySet<string>;
  onFold: (manifestId: string) => void;
};

export function ViewsColumn({ bands, view, onView, folded, onFold }: Props) {
  const store = useProjectsStore()!;
  const snap = useProjects();
  return (
    <section className="deck-panel deck-list projects-list" aria-label="Projects" tabIndex={-1}>
      <header className="deck-band">
        Projects
        <Tooltip label="Go to Projects" shortcut={PROJECTS_KEYS.list}>
          <Kbd>{PROJECTS_KEYS.list}</Kbd>
        </Tooltip>
      </header>
      <ul className="deck-list__rows projects-list__rows">
        <li>
          <ul className="deck-list__group projects-views" aria-label="Views">
            {VIEW_IDS.map((id, at) => {
              const { Glyph, said } = VIEW_MARKS[id];
              const here = snap.open === undefined && view === id;
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
        {bands.map((band) => {
          const open = !folded.has(band.manifestId);
          const Chevron = open ? ChevronDown : ChevronRight;
          const own = ownId(band.manifestId);
          return (
            <li key={band.manifestId} className="projects-band projects-repo" data-manifest={band.manifestId}>
              <div className="projects-repo__head">
                <button type="button" className="projects-repo__fold" aria-expanded={open} onClick={() => onFold(band.manifestId)}>
                  <Chevron size={16} strokeWidth={2} aria-hidden />
                  <span className="projects-repo__name">{band.label}</span>
                </button>
                <Tooltip label="New project" shortcut={capOf(PROJECTS_KEYS.newProject)} asChild>
                  <button type="button" className="projects-band__new" aria-label={`New project in ${band.label}`} onClick={() => store.name(band.manifestId)}>
                    <Plus size={16} strokeWidth={2} aria-hidden />
                  </button>
                </Tooltip>
              </div>
              {open ? (
                <ul className="deck-list__group">
                  <li className="deck-list__item">
                    <button type="button" className="projects-item" data-own={band.manifestId} data-selected={snap.open === own || undefined} aria-current={snap.open === own || undefined} aria-label={`Tasks of ${band.label}`} onClick={() => store.open(own)}>
                      Tasks
                    </button>
                  </li>
                  {snap.projects
                    .filter((one) => one.manifestId === band.manifestId && one.own !== true)
                    .map((one) => (
                      <li key={one.id} className="deck-list__item">
                        {snap.renaming === one.id ? (
                          <RenameProject id={one.id} name={one.name} />
                        ) : (
                          <button type="button" className="projects-item" data-project={one.id} data-selected={snap.open === one.id || undefined} aria-current={snap.open === one.id || undefined} onClick={() => store.open(one.id)}>
                            {one.name}
                          </button>
                        )}
                      </li>
                    ))}
                </ul>
              ) : null}
              {snap.naming === band.manifestId ? <NameProject band={band} /> : null}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

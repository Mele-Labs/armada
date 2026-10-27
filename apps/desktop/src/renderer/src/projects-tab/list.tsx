// His projects, a panel for each repository with its name as the head: every repository's on All, only the picked one's
// otherwise. A press on a head opens that repository's own tasks on the right; the `+` at its right asks a new project's
// name in that repository; a press on a project opens it.

import { useState } from "react";
import { Plus } from "lucide-react";
import { Kbd, Textarea, Tooltip, forgetKept } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf, repositoryLabel } from "@armada/shell";

import { PROJECTS_KEYS, capOf } from "./key-table";
import { ownId } from "./model";
import { useProjects, useProjectsStore } from "./store";

/** A repository's panel: its Manifest id, its checkout, and the name it is shown by. */
export type Band = { manifestId: string; root: string; label: string };

export function bandsIn(picked: RepositorySummary | null, repositories: readonly RepositorySummary[]): Band[] {
  return (picked === null ? repositories : [picked]).flatMap((one) => {
    const id = jobIdOf(one);
    return id === undefined ? [] : [{ manifestId: id, root: one.root, label: repositoryLabel(one, repositories) }];
  });
}

export function ProjectsList({ bands }: { bands: readonly Band[] }) {
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
        {bands.map((band) => (
          <li key={band.manifestId} className="deck-list__band projects-band" aria-label={band.label} data-manifest={band.manifestId}>
            <h3 className="deck-list__head projects-band__head" data-selected={snap.open === ownId(band.manifestId) || undefined}>
              <button type="button" className="projects-band__open" data-own={band.manifestId} data-selected={snap.open === ownId(band.manifestId) || undefined} aria-current={snap.open === ownId(band.manifestId) || undefined} onClick={() => store.open(ownId(band.manifestId))}>
                {band.label}
              </button>
              <Tooltip label="New project" shortcut={capOf(PROJECTS_KEYS.newProject)} asChild>
                <button type="button" className="projects-band__new" aria-label={`New project in ${band.label}`} onClick={() => store.name(band.manifestId)}>
                  <Plus size={16} strokeWidth={2} aria-hidden />
                </button>
              </Tooltip>
            </h3>
            <ul className="deck-list__group">
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
            {snap.naming === band.manifestId ? <NameProject band={band} /> : null}
          </li>
        ))}
      </ul>
    </section>
  );
}

/** A project's name, edited in its place in the list. Enter keeps it; Esc leaves it as it was. */
function RenameProject({ id, name }: { id: string; name: string }) {
  const store = useProjectsStore()!;
  const keep = `projects:rename:${id}`;
  const [words, setWords] = useState(name);
  return (
    <form
      className="projects-rename"
      aria-label={`Rename ${name}`}
      onSubmit={(event) => event.preventDefault()}
    >
      <Textarea
        rows={1}
        autoFocus
        keep={keep}
        aria-label="Project name"
        value={words}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            forgetKept(keep);
            if (words.trim() === "") store.renaming(undefined);
            else store.renameProject(id, words.trim());
            window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`.projects-item[data-project="${CSS.escape(id)}"]`)?.focus());
          }
        }}
      />
    </form>
  );
}

/** The new project's name, at the foot of its repository's panel. Enter adds it and opens it; Esc puts the field away. */
function NameProject({ band }: { band: Band }) {
  const store = useProjectsStore()!;
  const keep = `projects:name:${band.manifestId}`;
  const [name, setName] = useState("");
  const add = () => {
    if (name.trim() === "") return;
    store.addProject(band.manifestId, name.trim());
    forgetKept(keep);
    setName("");
  };
  return (
    <form
      className="projects-name"
      aria-label="New project"
      onSubmit={(event) => {
        event.preventDefault();
        add();
      }}
    >
      <Textarea
        rows={1}
        autoFocus
        keep={keep}
        aria-label="Project name"
        value={name}
        onChange={(event) => setName(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            add();
          }
        }}
      />
    </form>
  );
}

// The repositories the tab lists (every one on All, only the picked one's otherwise) and the two name fields of the left column:
// a project's new name and a new project's.

import { useState } from "react";
import { Folder } from "lucide-react";
import { Textarea, forgetKept } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import { jobIdOf, repositoryLabel } from "@armada/shell";

import { useProjectsStore } from "./store";

/** A repository's panel: its Manifest id, its checkout, and the name it is shown by. */
export type Band = { manifestId: string; root: string; label: string };

export function bandsIn(picked: RepositorySummary | null, repositories: readonly RepositorySummary[]): Band[] {
  return (picked === null ? repositories : [picked]).flatMap((one) => {
    const id = jobIdOf(one);
    return id === undefined ? [] : [{ manifestId: id, root: one.root, label: repositoryLabel(one, repositories) }];
  });
}

/** A project's name, edited in its place in the list, drawn as the row is with its words selected. Enter keeps it; Esc leaves it as it was. */
export function RenameProject({ id, name }: { id: string; name: string }) {
  const store = useProjectsStore()!;
  const [words, setWords] = useState(name);
  return (
    <form className="projects-rename projects-item" aria-label={`Rename ${name}`} onSubmit={(event) => event.preventDefault()}>
      <Folder size={16} strokeWidth={2} aria-hidden />
      <input
        className="projects-rename__input"
        autoFocus
        aria-label="Project name"
        value={words}
        onFocus={(event) => event.currentTarget.select()}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.nativeEvent.isComposing) {
            event.preventDefault();
            if (words.trim() === "") store.renaming(undefined);
            else store.renameProject(id, words.trim());
            window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`.projects-item[data-project="${CSS.escape(id)}"]`)?.focus());
          }
        }}
      />
    </form>
  );
}

/** The new project's name, at the foot of the projects. Enter adds it and opens it; Esc puts the field away. */
export function NameProject({ open = true }: { open?: boolean }) {
  const store = useProjectsStore()!;
  const keep = "projects:name";
  const [name, setName] = useState("");
  const add = () => {
    if (name.trim() === "") return;
    store.addProject(name.trim(), open);
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

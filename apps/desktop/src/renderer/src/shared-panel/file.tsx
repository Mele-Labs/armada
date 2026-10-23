// Any file in the side panel: read through main, edited in the code editor, saved back. The draft is kept by path, so closing
// the panel on an edit and opening the file again finds the edit, and a save is what clears it. A file that changed on disk
// while it was open is reloaded silently when there is no edit, and met at the save when there is one.

import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { FilePanel, FileView, isPressed, readKept, useKept } from "@armada/components";
import type { FilePanelProps } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import type { PanelFileItem, PanelFileRead } from "../../../shared/api/files";
import { openSessionIn } from "../open-session";
import { useBack } from "./panels";
import { closePanel, filePreviewing, openInPanel, useFilesBeside, type PanelFileMarked } from "./store";

/** What a read was refused for, as a fact. */
const NOT_OPENED = {
  not_addressable: "Not a file this panel opens",
  too_big: "Too big to open",
  binary: "Not a text file",
  unreadable: "Could not be read",
} as const;

type Loaded = { text?: string; bytes?: Uint8Array; type: string; version: string };

const nameOf = (path: string): string => path.split("/").pop() ?? path;

/** ⌘O, anywhere in a window that draws the panel: a dialog main opens, and the file it answers shown in the panel. */
export function useOpenFileKey(): void {
  useEffect(() => {
    function press(event: KeyboardEvent): void {
      if (event.repeat || event.defaultPrevented || !isPressed("open_file", event)) return;
      event.preventDefault();
      void window.armada.pickPanelFile().then((item) => {
        if (item !== null) openInPanel(item);
      });
    }
    window.addEventListener("keydown", press);
    return () => window.removeEventListener("keydown", press);
  }, []);
}

/** What a file's view needs of the file: read, kept as a draft, saved, and met again where the disk moved. The sheet and the inline view both draw it. */
function useFile(item: PanelFileItem): Omit<FilePanelProps, "onClose" | "floor" | "back"> {
  const [loaded, setLoaded] = useState<Loaded | undefined>(undefined);
  const [refused, setRefused] = useState<string | undefined>(undefined);
  const [moved, setMoved] = useState<{ onDisk: string; version: string } | null>(null);
  const [saving, setSaving] = useState(false);
  const [imageSrc, setImageSrc] = useState<string | undefined>(undefined);
  // The edit, by path. Null is no edit: the text is what was read.
  const [draft, setDraft, clearDraft] = useKept<string | null>(`file:${item.path}`, null);

  const read = useCallback(
    async (silently: boolean) => {
      const answer: PanelFileRead = await window.armada.readPanelFile(item);
      if (!answer.ok) {
        setRefused(NOT_OPENED[answer.why]);
        return;
      }
      if (!silently) setRefused(undefined);
      setLoaded({ ...(answer.text === undefined ? {} : { text: answer.text }), ...(answer.bytes === undefined ? {} : { bytes: answer.bytes }), type: answer.type, version: answer.version });
    },
    [item],
  );
  useEffect(() => void read(false), [read]);

  const text = draft ?? loaded?.text;
  const dirty = draft !== null && draft !== loaded?.text;
  // A change on disk under no edit is taken in; under an edit it is met at the save. **The edit is asked of the keeping, not of
  // this render**: a change that lands in the same moment as a keystroke would otherwise find the render behind it and reload under it.
  useEffect(
    () =>
      window.armada.onPanelFileChanged((changed) => {
        if (changed.path === item.path && readKept<string | null>(`file:${item.path}`) == null) void read(true);
      }),
    [item.path, read],
  );

  const bytes = loaded?.bytes;
  const type = loaded?.type;
  useEffect(() => {
    if (bytes === undefined || type === undefined || !type.startsWith("image/")) return;
    const address = URL.createObjectURL(new Blob([bytes.slice().buffer], { type }));
    setImageSrc(address);
    return () => URL.revokeObjectURL(address);
  }, [bytes, type]);

  const land = (answer: Awaited<ReturnType<typeof window.armada.savePanelFile>>, wrote: string) => {
    if (answer.state === "saved") {
      setLoaded((was) => ({ ...(was ?? { type: "text/plain" }), text: wrote, version: answer.version }));
      clearDraft();
      setMoved(null);
      setRefused(undefined);
    } else if (answer.state === "moved") {
      setMoved({ onDisk: answer.onDisk, version: answer.version });
    } else {
      setRefused(answer.detail);
    }
  };

  const save = (version: string | undefined) => {
    if (text === undefined || version === undefined) return;
    setSaving(true);
    void window.armada
      .savePanelFile(item, text, version)
      .then((answer) => land(answer, text))
      .finally(() => setSaving(false));
  };

  const openIn = (where: "editor" | "finder") =>
    void window.armada.openPanelFileIn(item, where).then((opened) => setRefused(opened.ok ? undefined : opened.detail));

  return {
    name: nameOf(item.path),
    path: item.path,
    type: loaded?.type ?? "text/plain",
    ...(text === undefined ? {} : { text }),
    ...(imageSrc === undefined ? {} : { imageSrc }),
    dirty,
    saving,
    onChange: (next) => (next === loaded?.text ? clearDraft() : setDraft(next)),
    onSave: () => save(loaded?.version),
    onOpenInEditor: () => openIn("editor"),
    onShowInFinder: () => openIn("finder"),
    moved: moved === null ? null : { onDisk: moved.onDisk },
    onKeepMine: () => save(moved?.version),
    onTakeDisk: () => {
      if (moved === null) return;
      setLoaded((was) => ({ ...(was ?? { type: "text/plain" }), text: moved.onDisk, version: moved.version }));
      clearDraft();
      setMoved(null);
    },
    ...(refused === undefined ? {} : { refused }),
  };
}

export function FileSide({ item }: { item: PanelFileItem & { marked?: PanelFileMarked } }) {
  const floor = useAtFloor();
  const back = useBack();
  const beside = useFilesBeside();
  const file = useFile(item);
  // Notes go to the Job that showed the file, or to the one whose page it was opened over; elsewhere the box asks which.
  const [job] = useState(() => item.sessionId ?? openSessionIn(document));
  return (
    <FilePanel
      {...file}
      onClose={closePanel}
      floor={floor}
      {...(back === undefined ? {} : { back: { label: back.label, onBack: back.onBack } })}
      {...(beside === undefined ? {} : { beside })}
      onPreviewing={filePreviewing}
      notes={{ key: `file:${item.path}`, ...(job === undefined ? {} : { job }) }}
      {...(item.marked === undefined ? {} : { marked: item.marked })}
    />
  );
}

/** The same file in the page, under `head`, for a surface that edits a file where it shows it. Nothing here closes. The path is the file, so a new path is a new view. */
export function FileInline({ path, from, head }: { path: string; from: PanelFileItem["from"]; head: ReactNode }) {
  const item = useMemo<PanelFileItem>(() => ({ kind: "file", path, from }), [path, from]);
  return <FileView {...useFile(item)} head={head} />;
}

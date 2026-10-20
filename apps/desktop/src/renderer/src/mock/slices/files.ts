// The file panel's reads and saves over a pretend disk, one per window. A save at a version the disk has moved past comes
// back `moved`, as main's will. A walk's `later` step changes the file last read from outside, once, so the next save meets it.
// ⌘O's dialog answers the Rust file, then the picture, and so on. Nothing is read from or written to the machine.

import type { FilesApi, PanelFileItem } from "../../../../shared/api/files";
import { CLOCK_FILE, PICTURE_FILE, pretendDisk } from "../files-disk";
import type { Slice } from "../fake-context";
import { alsoOnTimePassing } from "../time-passes";

export const files: Slice<FilesApi, Record<string, never>> = {
  name: "files",
  state: {},
  api: () => {
    const disk = pretendDisk();
    const listeners = new Set<(item: PanelFileItem) => void>();
    let last: string | undefined;
    let picked = 0;
    const versionOf = (n: number) => `v${n}`;
    return {
      readPanelFile: async (item) => {
        // A file a scenario's ledger names that this disk was not given reads as its own name.
        if (!disk.has(item.path)) disk.set(item.path, { text: `${item.path.split("/").pop()}\n`, type: "text/plain", version: 1 });
        const file = disk.get(item.path)!;
        last = item.path;
        // A walk's `later` step, from this window on. The latest window wins, as it does for a scenario's.
        alsoOnTimePassing(() => {
          const at = last === undefined ? undefined : disk.get(last);
          if (last === undefined || at === undefined || at.elsewhere === undefined) return;
          const { elsewhere, ...rest } = at;
          disk.set(last, { ...rest, text: elsewhere, version: at.version + 1 });
          const changed: PanelFileItem = { kind: "file", path: last, from: "settings" };
          listeners.forEach((one) => one(changed));
        });
        return { ok: true, ...(file.text === undefined ? {} : { text: file.text }), ...(file.bytes === undefined ? {} : { bytes: file.bytes }), type: file.type, version: versionOf(file.version) };
      },
      savePanelFile: async (item, text, version) => {
        const file = disk.get(item.path);
        if (file === undefined) return { state: "failed", detail: "The file is not on disk" };
        if (versionOf(file.version) !== version) return { state: "moved", onDisk: file.text ?? "", version: versionOf(file.version) };
        const next = file.version + 1;
        disk.set(item.path, { ...file, text, version: next });
        return { state: "saved", version: versionOf(next) };
      },
      pickPanelFile: async () => (picked++ % 2 === 0 ? CLOCK_FILE : PICTURE_FILE),
      openPanelFileIn: async () => ({ ok: true }),
      onPanelFileChanged: (onChanged) => {
        listeners.add(onChanged);
        return () => void listeners.delete(onChanged);
      },
    };
  },
};

/**
 * A path drawn as its folders, each one a press: `Users › a › Documents › notes`. Pressing a part
 * chooses that folder and everything under it. **A control, not a breadcrumb**: nothing here
 * navigates, and the design system's rule against breadcrumb trails is about where a screen is.
 *
 * The chosen part carries `aria-pressed`; the parts after it are drawn as covered by the choice.
 */
export type PathPickerProps = {
  /** An absolute path. */
  path: string;
  /** The folder chosen: `path` itself or a folder above it. */
  chosen: string;
  onChoose: (folder: string) => void;
  /** What the choice is for, read as the group's name. */
  label: string;
};

/** Each folder `path` passes through, from the first below the root to the path itself. */
export function foldersOf(path: string): { name: string; folder: string }[] {
  const parts = path.split("/").filter((part) => part.length > 0);
  return parts.map((name, at) => ({ name, folder: `/${parts.slice(0, at + 1).join("/")}` }));
}

export function PathPicker({ path, chosen, onChoose, label }: PathPickerProps) {
  const folders = foldersOf(path);
  const reached = folders.findIndex((one) => one.folder === chosen);
  return (
    <div className="armada-path-picker" role="group" aria-label={label}>
      {folders.map((one, at) => (
        <span key={one.folder} className="armada-path-picker__part">
          {at === 0 ? null : (
            <span className="armada-path-picker__between" aria-hidden>
              ›
            </span>
          )}
          <button
            type="button"
            className="armada-path-picker__folder"
            aria-pressed={one.folder === chosen}
            data-covered={reached >= 0 && at > reached ? true : undefined}
            title={one.folder}
            onClick={() => onChoose(one.folder)}
          >
            {one.name}
          </button>
        </span>
      ))}
    </div>
  );
}

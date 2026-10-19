/** Where a repository's runs are kept: the types, apart from the component so a .ts reader needs no JSX. */
export type RunsPlaceId = "mac" | "repository" | "synced" | "cloud";
export type RunsPlace = { id: RunsPlaceId; detail?: string };

/** A folder on this Mac that a service keeps in sync. */
export type SyncedFolder = { label: string; path: string };

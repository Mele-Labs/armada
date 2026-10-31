// What a page may ask the Job browser for, and what the person answers. Shared by the pane and, later, by what holds the choices.

export type SitePermission = "local_network" | "camera" | "microphone" | "notifications" | "clipboard" | "location" | "downloads";

export type Choice = "ask" | "allow" | "block";

/** The seven, in the order they are listed. */
export const SITE_PERMISSIONS: readonly { permission: SitePermission; label: string }[] = [
  { permission: "local_network", label: "Local network" },
  { permission: "camera", label: "Camera" },
  { permission: "microphone", label: "Microphone" },
  { permission: "notifications", label: "Notifications" },
  { permission: "clipboard", label: "Clipboard" },
  { permission: "location", label: "Location" },
  { permission: "downloads", label: "Downloads" },
];

export const CHOICES: readonly { choice: Choice; label: string }[] = [
  { choice: "ask", label: "Ask" },
  { choice: "allow", label: "Allow" },
  { choice: "block", label: "Block" },
];

export const labelOfPermission = (permission: SitePermission): string => SITE_PERMISSIONS.find((one) => one.permission === permission)?.label ?? permission;

/** The choices made for each host, the defaults beneath them, and the one request waiting on the person. Main keeps them for each repository. */
export type SiteSettings = {
  sites: Readonly<Record<string, Partial<Record<SitePermission, Choice>>>>;
  defaults: Readonly<Record<SitePermission, Choice>>;
  pending: { permission: SitePermission; host: string } | null;
};

export const NO_SETTINGS: SiteSettings = {
  sites: {},
  defaults: { local_network: "ask", camera: "ask", microphone: "ask", notifications: "ask", clipboard: "ask", location: "ask", downloads: "block" },
  pending: null,
};

/** What a host is given for a permission: its own choice, else the default. */
export const effectiveChoice = (site: Pick<SiteSettings, "sites" | "defaults">, host: string, permission: SitePermission): Choice =>
  site.sites[host]?.[permission] ?? site.defaults[permission];

/** What the person says to a request: Esc is `dismiss`, which refuses it now and keeps nothing. */
export type Answer = "allow" | "block" | "dismiss";

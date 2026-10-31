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

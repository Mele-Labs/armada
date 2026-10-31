// The choices made for each site, kept for each repository in one file so they outlast the app.

import { NO_SETTINGS, type Choice, type SitePermission, type SiteSettings } from "../shared/site-permissions";

type Kept = Pick<SiteSettings, "sites" | "defaults">;
type Disk = { read(): string | null; write(text: string): void };

export class SiteSettingsStore {
  private readonly repositories = new Map<string, { sites: Record<string, Partial<Record<SitePermission, Choice>>>; defaults: Partial<Record<SitePermission, Choice>> }>();

  constructor(private readonly disk: Disk) {
    try {
      const text = disk.read();
      const parsed: unknown = text === null ? {} : JSON.parse(text);
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        for (const [key, one] of Object.entries(parsed)) {
          if (typeof one !== "object" || one === null) continue;
          const { sites, defaults } = one as { sites?: unknown; defaults?: unknown };
          this.repositories.set(key, {
            sites: typeof sites === "object" && sites !== null ? { ...sites } : {},
            defaults: typeof defaults === "object" && defaults !== null ? { ...defaults } : {},
          });
        }
      }
    } catch {
      // A corrupt file starts again from the defaults.
      this.repositories.clear();
    }
  }

  settingsOf(key: string): Kept {
    const one = this.repositories.get(key);
    return { sites: one?.sites ?? NO_SETTINGS.sites, defaults: { ...NO_SETTINGS.defaults, ...one?.defaults } };
  }

  choose(key: string, host: string, permission: SitePermission, choice: Choice): void {
    const one = this.entry(key);
    one.sites[host] = { ...one.sites[host], [permission]: choice };
    this.save();
  }

  setDefault(key: string, permission: SitePermission, choice: Choice): void {
    this.entry(key).defaults[permission] = choice;
    this.save();
  }

  private entry(key: string) {
    let one = this.repositories.get(key);
    if (one === undefined) {
      one = { sites: {}, defaults: {} };
      this.repositories.set(key, one);
    }
    return one;
  }

  private save(): void {
    this.disk.write(JSON.stringify(Object.fromEntries(this.repositories)));
  }
}

// The choices made for each site, kept for each repository in one file so they outlast the app, and the one set of defaults beneath them for the
// whole Mac. A file from before the defaults were one set kept them for each repository: they are not read, and its sites are.

import {
  CHOICES,
  NO_SETTINGS,
  SITE_PERMISSIONS,
  type Choice,
  type SiteDefaults,
  type SitePermission,
  type SiteSettings,
} from "../shared/site-permissions";

type Disk = {
  read(): string | null;
  write(text: string): void;
};

/** Where the machine-wide defaults sit in the file, beside the repositories. No repository is named by it. */
const DEFAULTS = "defaults";

/** The saved defaults that are a permission and a choice, and nothing else. */
function validDefaults(saved: unknown): Partial<SiteDefaults> {
  if (typeof saved !== "object" || saved === null) return {};
  const kept: Partial<Record<SitePermission, Choice>> = {};
  for (const { permission } of SITE_PERMISSIONS) {
    const choice = (saved as Record<string, unknown>)[permission];
    if (CHOICES.some((one) => one.choice === choice)) kept[permission] = choice as Choice;
  }
  return kept;
}

export class SiteSettingsStore {
  private readonly repositories = new Map<
    string,
    {
      sites: Record<string, Partial<Record<SitePermission, Choice>>>;
    }
  >();
  private saved: Partial<SiteDefaults> = {};

  constructor(private readonly disk: Disk) {
    try {
      const text = disk.read();
      const parsed: unknown = text === null ? {} : JSON.parse(text);
      if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
        for (const [key, one] of Object.entries(parsed)) {
          if (key === DEFAULTS) {
            this.saved = validDefaults(one);
            continue;
          }
          if (typeof one !== "object" || one === null) continue;
          const { sites } = one as {
            sites?: unknown;
          };
          this.repositories.set(key, {
            sites:
              typeof sites === "object" && sites !== null
                ? {
                    ...sites,
                  }
                : {},
          });
        }
      }
    } catch {
      // A corrupt file starts again from the defaults.
      this.repositories.clear();
      this.saved = {};
    }
  }

  /** The defaults for every Job's browser: what was saved over the shipped ones, so a permission added since has its shipped value. */
  defaults(): SiteDefaults {
    return {
      ...NO_SETTINGS.defaults,
      ...this.saved,
    };
  }

  settingsOf(key: string): Pick<SiteSettings, "sites" | "defaults"> {
    return {
      sites: this.repositories.get(key)?.sites ?? NO_SETTINGS.sites,
      defaults: this.defaults(),
    };
  }

  choose(key: string, host: string, permission: SitePermission, choice: Choice): void {
    const one = this.entry(key);
    one.sites[host] = {
      ...one.sites[host],
      [permission]: choice,
    };
    this.save();
  }

  setDefault(permission: SitePermission, choice: Choice): void {
    this.saved = {
      ...this.saved,
      [permission]: choice,
    };
    this.save();
  }

  private entry(key: string) {
    let one = this.repositories.get(key);
    if (one === undefined) {
      one = {
        sites: {},
      };
      this.repositories.set(key, one);
    }
    return one;
  }

  private save(): void {
    this.disk.write(
      JSON.stringify({
        ...Object.fromEntries(this.repositories),
        [DEFAULTS]: this.saved,
      }),
    );
  }
}

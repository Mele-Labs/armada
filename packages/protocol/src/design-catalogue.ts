// `get_design_catalogue`: what a `mock` is drawn with, kept by Fleet per checkout. Mirrors
// `crates/ipc/src/design_catalogue.rs`.

/** Which part of the design catalogue a call asks for. `contents` answers names only. */
export type CatalogueArea = "contents" | "tokens" | "components" | "icons" | "patterns";

/** What a token is for, read from its name and value. */
export type TokenRole = "colour" | "space" | "type" | "radius" | "shadow" | "motion" | "other";

/**
 * One token: `value` resolved in the default theme, `alias` the `var()` it was written as, and `themes` another
 * theme's value where it differs.
 */
export type CatalogueToken = {
  name: string;
  value: string;
  alias?: string;
  themes?: Record<string, string>;
};

/** Tokens of one role sharing a name family, such as `--status`. */
export type TokenGroup = {
  role: TokenRole;
  family: string;
  tokens: CatalogueToken[];
};

/** A component: its file, the stylesheet beside it, its real class names, and HTML read from its own tags. */
export type CatalogueComponent = {
  name: string;
  file: string;
  sheet?: string;
  classes: string[];
  sample?: string;
};

/** A house pattern the repository keeps in `.armada/catalogue/patterns.html`. */
export type CataloguePattern = {
  name: string;
  note?: string;
  html: string;
};

/** One token group as the contents list it. */
export type TokenFamily = {
  role: TokenRole;
  family: string;
  tokens: number;
};

/** The names in each area of the catalogue. */
export type CatalogueContents = {
  tokens: TokenFamily[];
  components: string[];
  patterns: string[];
  icons: number;
};

/**
 * `GET /sessions/design_catalogue?area=&match=&repository=&session_id=`, the agent's. Only the asked area is
 * filled; `cut` says what was left out to stay under 24 KB. An empty array is the same as an omitted one.
 */
export type DesignCatalogue = {
  area: CatalogueArea;
  themes?: string[];
  sheets?: string[];
  contents?: CatalogueContents;
  tokens?: TokenGroup[];
  components?: CatalogueComponent[];
  icons?: string[];
  patterns?: CataloguePattern[];
  cut?: string;
};

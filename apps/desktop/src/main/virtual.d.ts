// Modules the main build makes up rather than reads from disk: `electron.vite.config.ts`.

declare module "virtual:armada-walk-finder" {
  /** The walk's page-side finder as one script, which defines `ArmadaFinder` where it runs. */
  const source: string;
  export default source;
}

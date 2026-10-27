// js-yaml ships no types and `@types/js-yaml` is not a dependency: the one
// call this package makes is typed here.
declare module "js-yaml" {
  export function load(text: string): unknown;
}

// The package a teammate sends in the walk `sharing-in-bridge`: what it pastes, and what its scenario reads back. Apart from the
// scenario because every export of a file in `scenarios/` is listed as a scenario.

/** The text a teammate sent: what a person pastes, and what Fleet reads below. */
export const PACKAGE = [
  "version = 1",
  "",
  "[[piece]]",
  'kind = "tile"',
  'list = "job"',
  'remote = "acme/widgets"',
  "",
  "[piece.tile]",
  'id = "deploy-preview"',
  'kind = "shell"',
  'name = "Deploy preview"',
  'text = "make preview"',
  "",
  "[[piece]]",
  'kind = "review_tool"',
  'remote = "acme/gadgets"',
  'id = "migration-safety"',
  'text = "Does any migration lock a table for long?"',
  "",
].join("\n");

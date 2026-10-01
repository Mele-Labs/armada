Every change to this repository carries the following, whatever the request
names. Doing them is part of the change and not scope expansion; leaving one
out is what makes a change unfinished.

- **Prose the change makes wrong is fixed in the same change.** A concept
  page, contract, practice doc, skill, comment or `armada.yml` note that would
  describe the old behaviour, or that points at something the change removes
  or renames, is updated or removed with it. `cargo xtask verify-foundations`
  refuses a document naming a repository path that does not exist, and a
  document under `docs/` that `docs/INDEX.md` does not list.
- **A new document under `docs/` is listed in `docs/INDEX.md`.**
- **The change carries the tests that prove it**, run through
  `armada check test` and the other Checks `armada.yml` declares for what it
  touches. A milestone's claim is proved by its acceptance test.
- **A generated file is regenerated and committed with its source.**
  `pnpm --filter @armada/desktop codegen` after a registry under
  `crates/core-model/domain/` or `protocol-version.toml` changes, and
  `cargo xtask verify-docs --write` for `docs/OPEN.md`.
- **Rust is formatted** with `cargo fmt --all`, and the build adds no warning
  `main` does not have.
- **What the change builds is reached.** A thing that exists and nothing
  calls, serves or draws is half-built, so wiring it to the place that uses it
  is part of the change. `docs/practices/half-built.md`.

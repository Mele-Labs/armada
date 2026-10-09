# Mod

**What it is:** A folder on this machine that changes how Bridge looks, written by a person or by a session, which Fleet reads and checks and which can never stop Fleet from starting or from listing.

---

**Kind:** Record.

You ask a session for a warmer dark theme. It makes a mod called `warm`, edits its colours and checks them; you pick `warm` in Settings, Theme, and Bridge redraws. A month later you ask for it in the repository, and it is on a branch under `packages/mods/<name>/`, ready to open as a pull request.

## What it is

> **Rule.** A mod is a folder under the machine's `mods/` directory, and nothing else is one.
> Why: a session that can write a file can make one, without a build and without touching Bridge's source.

> **Rule.** Fleet stores and checks a mod, and Bridge is what applies it.
> Why: Fleet never draws, and a theme is the only kind a mod can be so far.

> **Rule.** A broken mod is a row with `valid` false, and never a failure of Fleet's start or of `list_mods`.
> Why: the folder is written by anything on the machine, so every read of it is bounded and every fault is that mod's own.

> **Rule.** Each mod is a git repository of its own.
> Why: its history is kept where it was written, and promoting it is a copy of two files.

## Where it lives

`~/Library/Application Support/Armada/mods/<name>/`, beside `armada.db`.

| File | Holds |
|---|---|
| `mod.toml` | `name`, `kind`, `version`, `description` |
| `theme.css` | Token values Bridge injects |
| `.git/` | The mod's own history, one commit at scaffold |

`name` is a slug of 1 to 40 lowercase letters, digits and `-`, starting with a letter or digit. `dark`, `light`, `system` and `default` are refused, because Bridge's own themes answer to them.

## mod.toml

A flat file of `key = "value"` lines and `#` comments, read by hand: a table, an array or a number is a problem.

| field | type | required | default | meaning |
|---|---|---|---|---|
| `name` | string | yes | | The folder's name |
| `kind` | string | yes | | `theme` |
| `version` | string | yes | | Up to 32 letters, digits, `.`, `+`, `-` |
| `description` | string | no | none | Up to 200 characters |

Any other key is a problem. A `kind` this build does not have makes the mod invalid, so a valid mod's kind is always one Bridge can act on.

## theme.css

> **Rule.** A stylesheet is `:root` and `[data-theme]` blocks of `--token: value;` declarations, at most 32 KiB, and anything else makes the mod invalid.
> Why: a theme is read by whatever it is substituted into, so the accepted language is a whitelist and not a filter.

The tokens are the names in `packages/tokens/tokens.json`, compiled into Fleet, so a token added there is accepted by the next build.

| Accepted | Refused |
|---|---|
| `:root`, `[data-theme]`, `[data-theme="name"]`, `:root[data-theme="name"]` | Any other selector, a block in a block |
| A token name as the property | Any other property, any custom property that is not a token |
| Values of letters, digits, spaces and `# . , % + - * / ( ) _ ' " :` | `url(`, `image-set(`, `expression(`, `javascript:`, `data:`, `script` |
| `/* comments */` | `@import` and every other at-rule |
| | A backslash, `<`, `>`, `{`, `}`, `!`, a value over 512 characters |
| A regular file | A link, a pipe, a file that is not UTF-8 |

Every problem is returned with its line, up to 20.

## The operations

| Operation | Kind | A session may | Does |
|---|---|---|---|
| `list_mods` | query | yes | Every mod: kind, version, description, `enabled`, `valid`, `reason`, `changed_at` |
| `scaffold_mod` | command | yes | Makes the folder, a starter `theme.css`, `git init` and one commit |
| `validate_mod` | query | yes | Every problem found, and the checked text when there are none |
| `set_mod_enabled` | command | no | This machine's switch for one mod |
| `promote_mod` | command | no | Copies the mod onto a new branch of a repository |

`mods.changed` carries the whole list. Fleet rescans every two seconds and publishes it when the list differs from the last, and `scaffold_mod` and `set_mod_enabled` publish at once, so a file a session wrote reaches an open Bridge without a restart.

> **Rule.** Bridge injects the `css` that `validate_mod` returned and no other text.
> Why: what loads is then what passed, with no window between the check and a second read.

## What is kept

| What | Where | Default |
|---|---|---|
| A mod's switch | `mod_switches`, a row per mod, with the other preferences | On |
| The active theme | `theme` in `get_preferences`, saved by `save_preferences` with `text`: a built-in's id, `catalogue:<slug>` or a mod's name | `dark` |

A mod nobody switched is on. A switch only says whether the owner allows the mod, and `valid` says whether Bridge may act on it: a mod can be on and invalid.

> **Rule.** A mod becomes the theme only when the owner picks it.
> Why: choosing is the act that changes his screen, and a session can write a mod but not save a preference, which is Helm's on his ask and Bridge's.

## Promotion

> **Rule.** Promotion puts the mod on a local branch of the repository and pushes nothing.
> Why: landing it is the pull request's, as for any other change.

1. `promote_mod` refuses a mod that is invalid, and a repository with no `packages/` directory.
2. Fleet leases a slot from the repository's pool, with a branch `armada/mod-<name>-<milliseconds>` cut fresh from the base.
3. It writes the two checked files to `packages/mods/<name>/` in the slot, never the mod's `.git`.
4. It commits exactly those paths and gives the slot back.
5. It answers with the branch and the commit.

The owner's checkout is never written, and a branch that exists is never moved. Every slot being in use is a refusal to try again.

## Safe mode

> **Rule.** A mod that cannot be read, or that fails its checks, is never given to Bridge to apply.
> Why: Bridge applies only what `validate_mod` returned, and that is nothing for an invalid mod.

Fleet's half is above: a bad mod is a row with its reason, and `set_mod_enabled` and `save_preferences` are Fleet calls, so switching a mod off or saving `dark` works while Bridge cannot draw the mod.

`bridge --no-mods` is Bridge's half. Main loads the window with `?nomods`, and the window then lists no mod in Settings or on the Mods surface and never calls `validate_mod`, so no mod's CSS reaches the page. Dark, Light and the catalogue still choose. Safe mode does not touch the saved theme: a mod chosen before stays chosen for the next ordinary start, and a built-in or catalogue theme picked in safe mode is saved like any other.

## What Bridge does with a list

| The list says | The window shows | The preference |
|---|---|---|
| The chosen mod is on and valid | Its theme, from `validate_mod`'s `css` | Kept |
| The chosen mod is on and invalid, or its check fails | Dark | Kept, so the mod returns when it passes |
| The chosen mod is switched off, or gone | Dark | Saved as `dark` |
| A new mod, on by default | Nothing changes | Kept |
| No list has been read yet | Dark for a chosen mod | Kept, since "not read" is not "gone" |

Bridge saves the theme with `save_preferences` as `theme` with `text`. A catalogue theme is saved as `catalogue:<slug>`, the id Bridge gives it, and a mod's name has no `:`, so the two cannot meet. Promote shows the branch `promote_mod` named and nothing more, since nothing is pushed and no pull request is opened.

# Account

**What it is:** The GitHub account a machine signs in to, the profiles it joins, and what each profile syncs.

---

**Kind:** Entity.

Defines the account. The shared home it sets up is [Kit](kit.md); the registry its store holds is [Session](session.md).

## Sign-in

**Rule.** Armada signs in with GitHub's device flow, once per machine.

| Step | What happens |
| --- | --- |
| 1 | Bridge shows a short code |
| 2 | The person types it at `github.com/login/device` |
| 3 | Fleet polls GitHub and finishes |

The GitHub app needs the device flow enabled, and Armada holds no client secret.

**Rule.** The identity is the GitHub account's numeric id.

## What signing in sets up

Signing in sets these up and asks for nothing more. The store holds nothing about any repository.

| Set up | Holds |
| --- | --- |
| Identity | The account's numeric id and its login |
| Central store | Pull request owner registry and red main claims, as opaque rows |
| Shared home | The repository Kit's rows are compared against |

## Profiles

**Rule.** A machine joins one or more profiles, each tied to one GitHub login, and profiles never merge.

**Rule.** Any machine holds any number of profiles, and nothing stops it adding any account.

A work machine starts on its own work profile with nothing from the personal side. Adding the personal login there is a deliberate act, and every kind it syncs is off until the person turns it on.

## What syncs

**Rule.** What syncs is chosen per kind and per machine, and every kind is off until it is turned on.

The kinds are Kit's: Skills, Plugins, Agent file, Global instructions, Sub agents, Commands, MCP servers, the allowlist, Workflows and Memory.

**Rule.** Each synced item has one scope.

| Scope | Where the item lives |
| --- | --- |
| Global | Every project |
| Repo | One named repository |
| Private | This machine only, and it never leaves |

## In Bridge

| Where | What |
| --- | --- |
| Title bar menu | One account row (the login, or Sign in), Sleep, Settings with its key |
| Menu button | The moon while Sleep is on, an amber dot while signed out; the turn while a sign-in waits, the end's glyph once one failed |
| Account | One place that holds everything below and keeps a sign-in in flight when it is left: a page in the work area like Settings, reached from the menu row and from Settings, or a panel from a row in the left rail. Two takes until the owner chooses |
| Account contents | Device code, set-up rows, profiles |
| Sign-in that ended | Denied, expired, not reached or refused: Fleet's sentence beside a mark, and Try again |
| Sign-in not set up | Fleet's sentence naming the blank setting, and a button to each in Settings |
| Profiles | A chip each from two profiles, active one marked; Add profile always shown; Sign out on each |
| Signed out | Who left, struck through; the store's row named only where it stays |
| Home not made yet | The set-up row for the shared home is not set up, and What syncs is greyed with its reason |
| What syncs | A sheet over a profile added on purpose, one switch per kind |
| Repository picker | The rail's first panel, above Work, collapsed to a glyph with the rail |

The glyphs are registry entries marked Proposed: `packages/icons/icons/user-check.toml`, `packages/icons/icons/database.toml`, `packages/icons/icons/globe.toml`, `packages/icons/icons/folder-git-2.toml` and `packages/icons/icons/laptop.toml`.

## Built

**Built, on mock data.** The title bar menu, the device code, the set-up rows, the profiles and What syncs all draw from `draft.account` in the mock Fleet, and the walks `onboarding-home`, `onboarding-work` and `title-bar-menu` play them.

**Drawn in the mock, awaiting the owner's walk.** The account as a page (`account-page`) or a rail panel (`account-rail`), the sign-in that ended or was refused, signing out, and the shared home not made yet: the walks `sign-in-ended`, `sign-out` and `sign-in-no-home`, with `sign-in-no-home-hidden` as the second take of What syncs.

**Not built, everything behind it.** Nothing signs in, no store exists and Fleet serves none of it.

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
| Account contents | A page header with the title and this machine's name, then glass under a mono band: one tile for each profile, and under them what signing in turns on |
| Sign-in that ended | Denied, expired, not reached or refused: Fleet's sentence beside a mark, and Try again, inside its own tile in the amber frame |
| Sign-in not set up | Fleet's sentence naming the blank setting, and a button to each in Settings, in its own tile |
| Profiles | A tile each, always listed, even while another signs in: the login, a mark for this machine's own or added on purpose, the active one in the accent frame with Make active on the rest, the three set-up rows, What syncs on an added one, Sign out; Add profile is a dashed tile |
| Sign-in in flight | Its own dashed tile beside the profiles, labelled Adding: the address as a link button, the code large with a copy act, a waiting mark, the time left, Cancel. It becomes that profile's tile when it finishes |
| Turns on | Three tiles, Red main claims, Pull request owners and Shared home, dashed with a broken ring while off and framed with a check once the store (the first two) or the home (the third) exists |
| Signed out | The one act, Sign in with GitHub, in a dashed strip across the Profiles frame, with the Turns on tiles off beneath it, a Sets up tile of the three rows none yet, and a Never tile: nothing is written to your repositories or pull requests, your token is not kept, a work machine syncs nothing until you turn a kind on |
| Signed out profile | Who left, struck through in a dashed tile; the store's row named only where it stays |
| Home not made yet | The set-up row for the shared home is not set up, and What syncs is greyed with its reason |
| What syncs | A sheet over a profile added on purpose, one switch per kind |
| Repository picker | The rail's first panel, above Work, collapsed to a glyph with the rail |

The glyphs are registry entries marked Proposed: `packages/icons/icons/user-check.toml`, `packages/icons/icons/database.toml`, `packages/icons/icons/globe.toml`, `packages/icons/icons/folder-git-2.toml`, `packages/icons/icons/laptop.toml`, `packages/icons/icons/monitor.toml`, `packages/icons/icons/user-pen.toml`, `packages/icons/icons/copy.toml`, `packages/icons/icons/hammer.toml` and `packages/icons/icons/house.toml`.

## Built

**Built, on mock data.** The title bar menu, the device code, the set-up rows, the profiles and What syncs all draw from `draft.account` in the mock Fleet, and the walks `onboarding-home`, `onboarding-work` and `title-bar-menu` play them.

**Drawn in the mock, awaiting the owner's walk.** The account as a page (`account-page`) or a rail panel (`account-rail`), the sign-in that ended or was refused, signing out, and the shared home not made yet: the walks `sign-in-ended`, `sign-out` and `sign-in-no-home`, with `sign-in-no-home-hidden` as the second take of What syncs.

**Not built, everything behind it.** Nothing signs in, no store exists and Fleet serves none of it.

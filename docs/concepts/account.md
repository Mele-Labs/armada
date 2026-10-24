# Account

**What it is:** The GitHub account a machine signs in to, the profiles it joins, and what each profile syncs.

---

**Kind:** Entity.

Defines the account. The shared home it sets up is [Kit](kit.md); the registry its store holds is [Job](session.md).

## Sign-in

**Rule.** Armada signs in with GitHub's device flow, once per machine.

| Step | What happens |
| --- | --- |
| 1 | Bridge shows a short code |
| 2 | The person types it at `github.com/login/device` |
| 3 | Fleet polls GitHub and finishes |

The GitHub app needs the device flow enabled, and Armada holds no client secret.
**The flow asks for one scope, `read:org`**, which lets the token list the account's organisations and nothing else; the token is used once, at sign-in, and dropped.

**Rule.** The identity is the GitHub account's numeric id.

## What signing in sets up

Signing in sets these up and asks for nothing more than that read-only organisation scope.
The store names no repository: it holds a hash of each, and for a repository whose runs are kept in the cloud, the commands and workspace paths its Jobs ran, [Central store](../contracts/central-store.md).

| Set up | Holds |
| --- | --- |
| Identity | The account's numeric id, its login and the organisations it belongs to, read once. An account GitHub will not list them for keeps none, marked unread, and still signs in |
| Central store | Pull request owner registry and red main claims, as opaque rows, and the runs of a repository whose Kit place is `cloud`, as commands and workspace paths under its hash |
| Shared home | The repository Kit's rows are compared against |

## Profiles

**Rule.** A machine joins one or more profiles, each tied to one GitHub login, and profiles never merge.

**Rule.** Any machine holds any number of profiles, and nothing stops it adding any account.

A work machine starts on its own work profile with nothing from the personal side. Adding the personal login there is a deliberate act, and every kind it syncs is off until the person turns it on.

## Repositories

**Rule.** No profile is active. Each repository on the machine is bound to one profile, and the repository claims and syncs as that profile.

**Rule.** A repository is matched through GitHub.
At sign-in Fleet keeps the account's login and the organisations it belongs to, and a repository whose owner is one of them is matched to that profile.
The token is still not kept.

| Case | What happens |
| --- | --- |
| One profile matches | The repository is bound to it, marked matched |
| No profile matches, such as another person's repository you collaborate on | It waits unbound until chosen once |
| Two profiles match, such as an owner both belong to | It waits unbound until chosen once |
| The person chooses a profile, or moves a bound repository to another | It is bound to that profile, marked chosen |
| The person chooses None | It claims and syncs nothing |

**Rule.** A choice wins over a match, is kept per repository on this machine and names a profile by its account id.
It outlives the profile: after signing out of that profile the repository has no store, and is not handed to another profile that happens to match.

**Rule.** Organisations are read at sign-in only, so one joined later is matched after the next sign-in.
An organisation that restricts third-party apps is not listed, and its repositories wait unbound until chosen.

## What syncs

**Rule.** What syncs is chosen per kind and per machine, and every kind is off until it is turned on.

The kinds are Kit's: Mods, Skills, Plugins, Global instructions, Sub agents, Commands, MCP servers, the allowlist, Workflows and Memory.
Agent file is not a kind of its own; it is the global instructions.
The choice is kept in this machine's store for each profile, and a kind switched off has no rows in Kit. [Kit](kit.md), *The shared home*.

**Rule.** Each synced item has one scope. **Not built:** a kind is the unit today, and no item carries a scope.

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
| Account | One page in the work area like Settings, reached from the menu row and from Settings, that holds everything below and keeps a sign-in in flight when it is left |
| Account contents | A page header with the title and this machine's name, then glass under a mono band: one tile for each profile, and under them what signing in turns on |
| Sign-in that ended | Denied, expired, not reached or refused: Fleet's sentence beside a mark, and Try again, inside its own tile in the amber frame |
| Sign-in not set up | Fleet's sentence naming the blank setting, and a button to each in Settings, in its own tile |
| Profiles | A tile each, always listed, even while another signs in: the login, a mark for this machine's own or added on purpose, the three set-up rows, the repositories bound to it (mono, scrolling inside the tile, each marked matched or chosen, with a Move menu), What syncs on an added one, Sign out; Add profile is a dashed tile |
| Sign-in in flight | Its own dashed tile beside the profiles, labelled Adding: the address as a link button, the code large with a copy act, a waiting mark, the time left, Cancel. It becomes that profile's tile when it finishes |
| Not matched | A frame of its own, drawn only when something is in it: each repository no profile owns, with a Choose menu listing the profiles and None; one given None keeps its row, marked |
| Turns on | Three tiles, Red main claims, Pull request owners and Shared home, dashed with a broken ring while off and framed with a check once the store (the first two) or the home (the third) exists |
| Signed out | The one act, Sign in with GitHub, in a dashed strip across the Profiles frame, with the Turns on tiles off beneath it, a Sets up tile of the three rows none yet, and a Never frame at the foot of the page, signed in or out: nothing is written to your repositories or pull requests, your token is not kept, a work machine syncs nothing until you turn a kind on |
| Signed out profile | Who left, struck through in a dashed tile; the store's row named only where it stays |
| Home not made yet | The set-up row for the shared home is not set up, and What syncs is greyed, tooltip No shared home yet |
| What syncs | A sheet over a profile added on purpose, one switch per kind |
| Repository picker | The top of the Cockpit's left column; at the title row's narrow width its entries are in the title menu |

The sign-in button carries the forge's own mark, never recoloured: white on a dark ground and black on a light one, as the forge's logo policy asks.
No token holds pure white or black, so `ForgeMark.css` is the one stylesheet that opts out of the colour-literal rule.

The glyphs are registry entries marked Proposed: `packages/icons/icons/user-check.toml`, `packages/icons/icons/database.toml`, `packages/icons/icons/globe.toml`, `packages/icons/icons/folder-git-2.toml`, `packages/icons/icons/laptop.toml`, `packages/icons/icons/user-pen.toml`, `packages/icons/icons/circle-dashed.toml`, `packages/icons/icons/ban.toml`, `packages/icons/icons/copy.toml`, `packages/icons/icons/hammer.toml` and `packages/icons/icons/house.toml`.

## Built

**Built, on mock data.** The title bar menu, the device code, the set-up rows, the profiles and What syncs all draw from `draft.account` in the mock Fleet, and the walks `onboarding-home`, `onboarding-work` and `title-bar-menu` play them.

**Drawn in the mock, awaiting the owner's walk.** The account page (`account-page`), the repositories bound to each profile and the ones that wait (`account-repositories`), the sign-in that ended or was refused, signing out, and the shared home not made yet: the walks `sign-in-ended`, `sign-out` and `sign-in-no-home`.

**Fleet serves binding.** `get_accounts` carries each profile's repositories and the ones bound to none, and `set_repository_profile` chooses a profile or None for a repository; Bridge draws them.
There is no active profile and no set-active act.

**Built in Fleet.** What syncs, per kind and per machine and kept in the store, and the shared home: Fleet makes the repository, and the sign-in's home step reads done once the home exists. [Kit](kit.md), *The shared home*, and [the protocol page](../practices/protocol/the-shared-home.md).

**Built in Bridge.** What syncs reads a profile's kinds from `get_home_kinds` when it opens and switches one with `set_home_kind`, drawn at once and then as Fleet answers; the home chip is made once this machine holds a copy of that profile's home.
A refused switch goes back and is said on the page, as the other refusals are.
**Not built.** Per-item scopes: Fleet sends no item of a kind, so a kind turned on lists none, and nothing shows it pulling.
The central store is deployed outside this repository, [Central store](../contracts/central-store.md).

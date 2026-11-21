# Account

**What it is:** The GitHub account a machine signs in to, the profiles it joins, and which store each project joins as which profile.

---

**Kind:** Entity.

Defines the account. What crosses between his Macs is [Kit](kit.md); the registry its store holds is [Job](session.md).

## Sign-in

**Rule.** Armada signs in with GitHub's device flow, once per machine.

| Step | What happens |
| --- | --- |
| 1 | Bridge shows a short code |
| 2 | The person types it at `github.com/login/device` |
| 3 | Fleet polls GitHub and finishes |

**The GitHub app sign-in uses is registered with one press in Settings, then one tick.**
The press opens the browser on a page Fleet serves, which posts the app's manifest to GitHub, and the person presses Create there.
GitHub sends the browser back to Fleet, which keeps the app's client id in `account.clientId` and never reads the client secret, private key or webhook secret that come with it.
The browser then lands on the app's settings page, where the person ticks Enable Device Flow, the one setting a manifest cannot carry.
Armada holds no client secret.
**The flow asks for one scope, `read:org`**, which lets the token list the account's organisations and nothing else; the token is used once, at sign-in, and dropped.

**Rule.** The identity is the GitHub account's numeric id.

## What signing in sets up

Signing in sets these up and asks for nothing more than that read-only organisation scope.
The store names no repository: it holds a hash of each, and for a repository whose runs are kept in the cloud, the commands and workspace paths its Jobs ran, [Central store](../contracts/central-store.md).

| Set up | Holds |
| --- | --- |
| Identity | The account's numeric id, its login and the organisations it belongs to, read once. An account GitHub will not list them for keeps none, marked unread, and still signs in |
| Central store | Nothing at sign-in. A project's store setting joins the store it names, as the login the project picks, [A store per project](../contracts/central-store.md#a-store-per-project). The store keeps the pull request owner registry and claims of every kind, as opaque rows naming who holds each, and the runs of a repository whose Kit place is `cloud`, as commands and workspace paths under its hash |
| Shared home | The repository Kit's rows are compared against. Retired: Kit keeps his Macs the same over the Mac link, [Kit](kit.md), *Over the Mac link*. Fleet no longer reads it since 19 Nov 2026 |

## Profiles

**Rule.** A machine joins one or more profiles, each tied to one GitHub login, and profiles never merge.

**Rule.** Any machine holds any number of profiles, and nothing stops it adding any account.

A work machine starts on its own work profile with nothing from the personal side.
Adding the personal login there is a deliberate act, and nothing of Kit crosses until the person switches a kind on, [Kit](kit.md), *Over the Mac link*.

## What a login does

**Rule.** A login proves who the person is to a store, and vouches a new Mac or phone, [Pocket](pocket.md), *Through his store*.
It no longer names Kit's home: Kit crosses over the Mac link, and the repository `<login>/armada-home` is retired, [Kit](kit.md), *The shared home, retired*.
**Not built.** Fleet still makes and reads that repository for each profile.

## Which login a project joins as

**Rule.** No profile is active, and no repository is bound to a profile by its owner.
**A project picks which GitHub login (profile) joins its store; a single repository may override it.**

| Case | What happens |
| --- | --- |
| The project names a login | Every repository of the project calls its store as that profile |
| A repository names its own login | That repository calls as it, and the project's pick is left for the others |
| The project has a store and names no login | It sends nothing, and says so, [A store per project](../contracts/central-store.md#a-store-per-project) |

**Rule.** A repository's override is kept per repository on this Mac and names a profile by its account id.
It outlives the profile: after signing out of that profile the repository has no store, and is not handed to another profile.

A project's pick is `set_project_store` with `account_id`, and a repository's override is `set_repository_profile`.
Matching a repository's owner to a login or an organisation binds nothing, and `get_accounts` sends `unmatched` empty.
A project file written before the login was named takes the one profile here already enrolled at its store, [A store per project](../contracts/central-store.md#a-store-per-project).

## What syncs

**What syncs is Kit's, not the account's.**
Each kind is switched on per Mac, and what crosses goes over the Mac link, [Kit](kit.md), *Over the Mac link*.
A project he shares, with its tasks, crosses the same link, [Project](project.md), *Sharing with his other Macs*.
Fleet keeps the choice per Mac in its store since 19 Nov 2026; the per-profile choice it kept before is not carried over.

## In Classic

This is Armada Classic's screen, and it stays as it is.

| Where | What |
| --- | --- |
| Title bar menu | One account row (the login, or Sign in), Settings with its key |
| Menu button | An amber dot while signed out; the turn while a sign-in waits, the end's glyph once one failed |
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
| Sync your Kit | A frame of three steps, signed in or out: Sign in here, Sync on Kit, Add to this machine (done on the other machine, so it never reads done), each with its reason in one line; the next undone step carries the button, Open Kit on the second |
| Home not made yet | The set-up row for the shared home is not set up, naming the repository the first Sync on Kit will make and its tooltip saying it is private and made through `gh`; What syncs is greyed, its tooltip saying the same. Step two of Sync your Kit echoes the same name. Where it is made is chosen on the Kit page's Sync bar, kept by Fleet, and shown read-only once the home exists or another Mac made it |
| What syncs | A sheet over a profile added on purpose, one switch per kind |
| Repository picker | The top of the Cockpit's left column; at the title row's narrow width its entries are in the title menu |

The sign-in button carries the forge's own mark, never recoloured: white on a dark ground and black on a light one, as the forge's logo policy asks.
No token holds pure white or black, so `ForgeMark.css` is the one stylesheet that opts out of the colour-literal rule.

The glyphs are registry entries marked Proposed: `packages/icons/icons/user-check.toml`, `packages/icons/icons/database.toml`, `packages/icons/icons/globe.toml`, `packages/icons/icons/folder-git-2.toml`, `packages/icons/icons/laptop.toml`, `packages/icons/icons/user-pen.toml`, `packages/icons/icons/circle-dashed.toml`, `packages/icons/icons/ban.toml`, `packages/icons/icons/copy.toml`, `packages/icons/icons/hammer.toml` and `packages/icons/icons/house.toml`.

## In native Bridge

**Settings › This Mac › Account is one page with three stacked sections, and the title bar's Account row opens it.**
**Not built.**

| Section | What it holds |
| --- | --- |
| Who you are | A card for each login, with Sign in through the device code and Sign out, and an Add a GitHub login card |
| Your devices | This Mac with its address and tunnel, the linked Macs and the phones, each with Link and Forget. Moved here from Settings › Bridge › Macs |
| What it turns on | A Kit status row first, and one row for each project store naming the login that joins it, each opening where it is set. Kit's row says In step with the linked Macs' names, Differs from the Macs it differs from in the caution colour, or Stays on this Mac where none is linked, and opens Kit. It reads the Mac link as Kit does, [Kit](kit.md), *Over the Mac link* |

Sign-in is the device code above, once per login, and a project's store row is where the project's pick and a repository's override are set, [Which login a project joins as](#which-login-a-project-joins-as).
Linking a Mac is as [Native Bridge](native-bridge.md), *Macs found*, says of Settings › Macs, and its list moves to this page.

## Built

**Built, on mock data.** The title bar menu, the device code, the set-up rows, the profiles and What syncs all draw from `draft.account` in the mock Fleet, and the walks `onboarding-home`, `onboarding-work` and `title-bar-menu` play them.

**Drawn in the mock, awaiting the owner's walk.** The account page (`account-page`), the repositories bound to each profile and the ones that wait (`account-repositories`), the sign-in that ended or was refused, signing out, and the shared home not made yet: the walks `sign-in-ended`, `sign-out`, `sign-in-no-home` and `sync-your-kit`.

**Fleet serves binding.** `get_accounts` carries each profile's repositories and the ones bound to none, and `set_repository_profile` chooses a profile or None for a repository; Bridge draws them.
There is no active profile and no set-active act.

**Built in Fleet.** What syncs, per kind and per item and per machine and kept in the store, and the shared home: Fleet makes the repository where the person chose, keeps that choice per profile, and the sign-in's home step reads done once the home exists.
Since 19 Nov 2026 the home is not read, so `get_accounts` names no home and the home step does not read done.
`get_accounts` carries each profile's organisations, read at sign-in, and where its home is or will be. [Kit](kit.md), *The shared home, retired*, and [the protocol page](../practices/protocol/the-shared-home.md).

**Built in Bridge.** What syncs reads a profile's kinds from `get_home_kinds` when it opens and switches one with `set_home_kind`, drawn at once and then as Fleet answers; the home chip is made once this machine holds a copy of that profile's home.
A refused switch goes back and is said on the page, as the other refusals are.
Kit switches the same kinds, and single items with `set_home_item`. [Kit](kit.md), *The shared home, retired*.
Since 19 Nov 2026 Fleet answers every kind off and refuses each switch, naming the link; Classic's What syncs is not moved onto `set_kit_link_kind`.
**Not built.** Per-item scopes: Fleet sends no item of a kind, so a kind turned on lists none, and nothing shows it pulling.
The central store is deployed outside this repository, [Central store](../contracts/central-store.md).

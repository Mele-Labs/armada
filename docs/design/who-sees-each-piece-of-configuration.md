# Who sees each piece of configuration a person keeps for a repository

**Every piece is yours unless you commit it to the repository, and yours wins wherever the two name the same thing.**
Sharing is a package you send someone, a file or the same text, and never a place Armada keeps.

The pieces are flight-control tiles, how a repository runs (`armada.yml`), workflows and review tools.
Armada's store keeps runs only.

## The failure this answers

You work in a private work repository with colleagues who do not use Armada.
You add a tile *Run tests* that runs `make test`.
Today you cannot say who sees it, whether it can follow you to another Mac without the repository's name landing in your iCloud Drive, or how a colleague who wants it gets it.

Each piece answers differently today:

| Piece | Where it lives on `main` | Who sees it |
|---|---|---|
| Tiles | `~/.armada/flight-controls.toml`, every repository at once (`docs/concepts/kit.md`, *Flight controls*) | You |
| How a repository runs | `armada.yml` at the workspace root, committed (`docs/contracts/system-architecture.md`) | Everyone with access to the repository |
| Workflows | Armada's, `~/.armada/workflows/`, `.armada/workflows/` (`docs/concepts/workflow.md`, *Where a definition comes from*) | The place decides |
| Review tools | Armada's, `~/.armada/review-tools/`, `.armada/review-tools/` (`docs/concepts/pull-review.md`, *Review tools*) | The place decides |

Pull request #2627 keeps a repository's tiles for you alone, beside its runs.
The branch `armada/flight-tiles-in-repository` (`58f2cbc2c`) commits them to `.armada/flight-controls.toml` on a Job's branch, and is parked for this design.

## The design

### Every piece takes the same places

| Place | Kept in | Who sees it |
|---|---|---|
| Armada's | The binary | Everyone |
| Yours | Your Kit: this Mac, a synced folder you named, or a bucket you own | You, on every Mac your Kit reaches |
| The repository's | `.armada/` in the repository, or `armada.yml` at its root, committed | Whoever the repository lets read it |

**A new piece is yours.**
Nothing reaches the repository or another person until you choose to commit it or share it.

**`armada.yml` can be yours too.**
A repository whose team does not use Armada runs from your copy, kept in Kit as `manifests/<owner--name>.yml`, and nothing is committed to it.
`onboard-repo` asks which place before it writes.

### Yours wins, and says so

When yours and the repository's name the same thing, yours runs for you, and Bridge marks the piece as hiding the repository's.
The name is the id each concept already uses: a workflow's `workflow_id`, a review tool's dashed name, a tile's id, and a Command's or Check's name in `armada.yml`.
Two copies that are the same carry no mark, because nothing is hidden.

### Commit to the repository is an option on a piece

**Commit to repository** starts a Job of its own: its own worktree and branch, one commit adding the piece under `.armada/`, and a pull request for that alone, landed like any other.
It works with no Job open.
The owner decided this on 21 Oct 2026, over committing on the branch of the Job you are in, where the piece rode in that Job's pull request with unrelated work.

Committing copies the piece.
Yours stays, and carries no mark while the two are the same.

### Share is a package

**Share** makes one file, `<name>.armada.toml`, from one or several pieces you pick.
**Copy** puts the same text on the clipboard.
You send it any way you like: a chat message, an email, a gist.

The package holds each piece as it is written, and the remote of the repository a piece is kept for.
It names nothing else about you or the repository.

**Your colleague drops the file on Bridge, or pastes the text.**
Import lists every piece and every command it would add, and **Add** is the one ask.
What they add becomes theirs, in their own Kit, and runs like their own.

A piece kept for a repository lands on their copy of it, matched by remote.
One whose remote they do not have waits unbound until they choose a repository once, as `docs/concepts/account.md` already does for an unmatched repository.

### A tile a teammate committed asks before it runs

A shell tile that came from the repository asks once before its first run on your Mac, and shows its command.
It asks again whenever its command changes, so an approved tile cannot later run something else.

The ask covers tiles only.
A repository's `armada.yml` already runs its Commands and Checks, as its own scripts do.
Workflows and review tools are prompts to agents that already work in that repository.

### A repository can be kept on this Mac only

Any repository can be marked **Keep on this Mac**.
Every piece of yours for it, and its run memory, stays in `~/.armada` on this Mac, whatever place the rest of your Kit uses.
Its run memory place is then `this_mac`, and no other is offered.

This is for a private work repository.
Without the mark, its name in a file name and its tiles' commands land in your synced folder or bucket in the clear.

## What it costs

| Cost | Paid by |
|---|---|
| `armada.yml` gets a second home, and every reader of a manifest resolves two places | Fleet's manifest loading, and `onboard-repo` |
| You can drift from your team without noticing, if you never read the mark | You |
| Yours can replace a Check the repository declares, for your own Jobs | The gate on your own Jobs; CI still runs the repository's |
| A shared command is readable by anyone who sees the message it travelled in | The person who shares it |
| Someone who presses Add without reading is never asked again | The person who imports it |
| A repository kept on this Mac does not follow you to another Mac | You, on the second Mac |
| A committed piece leaves your copy behind until you remove it | You |
| Each piece committed is one more Job and one more pull request | You, and whoever reviews the repository's pull requests |

## What it rules out

| Alternative | Why it loses |
|---|---|
| Share as a commit on a Job's branch | Sharing with one colleague would mean a pull request the whole team reviews. Committing stays, as its own option |
| A team space in Armada's store, or a team bucket | `docs/contracts/central-store.md`, *Rows*, keeps runs and opaque hashes only, and a private repository's commands would sit in a service its owners never agreed to |
| A share link served by Armada | It needs a server holding the piece, which is the team space by another name |
| The repository's copy wins | A teammate's commit would replace your setup without a word |
| Hashed file names in Kit, in place of Keep on this Mac | The commands stay in the clear, so it hides less than it looks like it does |
| Asking before every repository piece runs, not only tiles | `armada.yml` already runs the repository's commands, and asking about it would ask about the repository's own build |

## What this changes that is already written

| Where | Says now | Becomes |
|---|---|---|
| `docs/concepts/workflow.md`, *Where a definition comes from* | The repository's `.armada/workflows/` is most specific, and wins | Yours wins, marked |
| `docs/concepts/pull-review.md`, *Review tools* | `<repository>/.armada/review-tools/` wins over Kit's | Yours wins, marked |
| `docs/concepts/kit.md`, *Flight controls*, as #2627 lands it | A repository's tiles are never committed into it, and sharing is not built | Commit and Share, as above |
| `docs/contracts/system-architecture.md` | An `armada.yml` lives at the workspace root, version-controlled | The repository's lives there, and yours may live in Kit |

Each is changed by the pull request that builds its part, not by this one.

## What would make this wrong

**That the repository is the only shared space a team wants.**
If teams want one set of pieces across many repositories without committing to each, this design has nowhere to put it, and a team space comes back.

**That a person reads what they import.**
Add is the one ask, so a package from someone careless is trusted on one press.

Out of scope: Triggers keep their own levels (`docs/concepts/trigger.md`), and a machine's settings stay in `settings.json`.

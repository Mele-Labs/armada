# Who sees each piece of configuration a person keeps for a repository?

**Decided 2026-10-19.** His framing: some things you want to share with the people you collaborate with, some you want kept locally, and some you keep locally but want to share easily, and private repositories matter. On Share he answered: *"share is literally something that can be packaged up and sent to someone. Nothing more than that right now. If I want to commit it to a repo, I can but thats an option."*

**Chosen: yours or the repository's, chosen per piece, and Share is a package.** Written up in `docs/design/who-sees-each-piece-of-configuration.md`.

| Asked | He chose | The cost he took |
|---|---|---|
| Which pieces take Armada's, yours and the repository's places | All four: tiles, `armada.yml`, workflows, review tools | `armada.yml` gets a second home |
| When yours and the repository's name the same thing | Yours wins, and Bridge marks it | Reverses the workflow and review-tool rule that the repository's wins |
| What Share does | Packages pieces to send someone, a file or the same text; committing is a separate option | The commands travel readable in whatever message carries them |
| When an imported command is asked about | Import lists it, and Add is the ask; a teammate's committed tile asks before its first run | Someone who presses Add unread is not asked again |
| A private repository in a synced folder or bucket | A repository can be kept on this Mac only | Its pieces do not follow him to another Mac |

**What it overrode.** The take he offered as a starting point made Share a commit on a Job's branch; he split that into Share and Commit.

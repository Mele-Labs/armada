# A queued idea

**Part of:** [Job](../session.md).

**A Job can be kept as an idea that does not start.** The dispatcher's Job kind offers Queue for later beside Start (`queue_session`, `⇧⌘↵`, rebindable), and its Model and Effort selects set what the idea will start on. Pressing it clears the draft and keeps `{ text, root, repository, model?, effort? }`. It is not a Job: no row on Fleet, no slot, no process, no proposal call.

**It lives on this machine, in Bridge.** The ideas are kept in `localStorage` (`armada.bridge.queued-ideas`, `queued-ideas.ts`), oldest first, since Fleet's preference set is closed and holds nothing of the kind. Another Bridge window hears the change through the `storage` event. A restart of Bridge or of Fleet keeps them.

**A queued idea sits at the head of its repository's band on the Cockpit deck's list**, dashed, with two acts and no other: Start and Remove. A click on its words starts nothing; `↵` on the row starts it.

**Start sends the idea through `StartSession` like any start**, with `request` the idea's words and `model` and `effort` only where the idea has them, so Fleet's proposer fills what is unset and never overwrites what was picked. The idea is removed once the Job's id exists, and a start that fails keeps it. The words go to the new Job as its first message.

# Where does this machine keep its settings and a person's Bridge preferences?

**Decided 2026-10-10, by the owner.** This reverses #927, which kept a person's Bridge preferences in Fleet's store, and he approved the reversal knowing that.

The limits, preferences and theme were rows in Fleet's database, and every other number Fleet runs by was a constant in `crates/armada/src/serve.rs` or an environment variable. Changing any of them meant a release, a variable, or a control Bridge happened to have.

**Chosen: one `settings.json` in the machine directory, VS Code style.**

- **Where:** `settings.json` beside the store, the parent of `fleet::runtime::machine_path()`. Not `~/.armada`, which `docs/contracts/system-architecture.md` keeps free of machine data.
- **What:** one flat JSON object of dotted camelCase keys, `"limits.dronesAtOnce": 3`. A key absent is the shipped default, and only a key a person changed is written. Fleet writes it sorted and pretty, beside itself and renamed over.
- **Who edits it:** Bridge, through `save_settings`, and a person by hand. Fleet watches the file and takes a save without a restart wherever the setting can move live.
- **What it refuses:** an unknown key, a wrong type or a value out of range refuses the whole file. The last good settings stay in force, and the refusal names the key on the console and on the wire, `armada.yml`'s rule.
- **Precedence:** an environment variable, then the file, then what ships. A Job's, a step's or a repository's own value wins over the machine's exactly as before.
- **One table:** `config::settings` lists every key, its kind and bounds, its default and whether it is live. Bridge draws its Settings page from what the table says.
- **Drone prompts are settings too.** A prompt's shipped text is its default and an edit is an override in the file. They arrive in a later change; the table has the kind for them.
- **The store's rows move once.** The first start that finds no file carries the saved limits and preferences into it, and the rows are never read again. The tables are not dropped.
- **A mod's on-off switch stays in the store.** It is the mod's state, not a setting.

**One pull request** carries the whole of it: prompts, limits, harness, models, effort, features, editor, terminal and timeouts.

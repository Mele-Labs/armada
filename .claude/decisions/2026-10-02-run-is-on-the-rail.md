# Where does a Studio's Run control go?

**Decided 2026-10-02.**

His annotation `20261002-042804-8gp7`, on the `Run` card at the whiteboard's top-right: *"Why is run not in the vertical toolbar on the left side?"*

**Chosen: "Yes, in 'what you place'."** *"A Run icon goes beside Note, Link, Sketch and the rest. Pressing it opens the list of commands, and the Run node lands where you're looking."*

- **The press opens the same menu the card did**: Checks and Commands, then servers. The node lands where the person is looking, through `useStudioPlacement`, as it did from the card.
- **Off says why, and is never hidden.** While the Studio is read-only, Run is drawn off and its tooltip says *Continue this Studio to run something.* Where the checkout declares nothing to run, it is off and says so.
- **Note, Link and Sketch are greyed too.** Asked whether they should stay hidden on a read-only Studio, he chose *"Land it; show all greyed"*: *"on a read-only Studio show all four greyed out with the same 'Continue this Studio…' tooltip, so the rail looks the same in both modes."* Theirs reads *Continue this Studio to add to it.*, and the tooltip carries no shortcut while the key is dead. **Cost he took:** *"four dead buttons whenever you reread an old Studio."*
- **The glyph is `zap`, minted.** Its row in `packages/icons/icons.toml` gives the reasons, including why `play` was not taken.
- **The top-right card is gone.**

**Cost he took:** Run needs a new icon in the registry, and while the Studio is read-only it is drawn off with a tooltip instead of being hidden.

**Where it landed:** `bridge/run-on-the-studio-rail`.

**Then, the same day**, asked about the palette: *"Add it, with R as the original design drew it."* Run has a row in `crates/core-model/domain/actions.toml`, `start_studio_run`, bound to `R`, shifted for `add_note`'s reason. The key and the palette row open the rail's menu rather than starting anything, and both are off exactly when Run is drawn off: `R` is dead and the row is dimmed with Run's own reason.

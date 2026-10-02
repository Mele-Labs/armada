# Where does a Studio's Run control go?

**Decided 2026-10-02.**

His annotation `20261002-042804-8gp7`, on the `Run` card at the whiteboard's top-right: *"Why is run not in the vertical toolbar on the left side?"*

**Chosen: "Yes, in 'what you place'."** *"A Run icon goes beside Note, Link, Sketch and the rest. Pressing it opens the list of commands, and the Run node lands where you're looking."*

- **The press opens the same menu the card did**: Checks and Commands, then servers. The node lands where the person is looking, through `useStudioPlacement`, as it did from the card.
- **Off says why, and is never hidden.** While the Studio is read-only, Run is drawn off and its tooltip says *Continue this Studio to run something.* Where the checkout declares nothing to run, it is off and says so. Note, Link and Sketch are still not drawn on a read-only Studio, so there Run is the only act in its group.
- **The glyph is `zap`, minted.** Its row in `packages/icons/icons.toml` gives the reasons, including why `play` was not taken.
- **The top-right card is gone.**

**Cost he took:** Run needs a new icon in the registry, and while the Studio is read-only it is drawn off with a tooltip instead of being hidden.

**Where it landed:** `bridge/run-on-the-studio-rail`.

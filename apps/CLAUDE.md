# apps/

Pocket (`pocket/`), the phone app, and the central store (`store/`). The desktop
Bridge is native and lives in `crates/bridge-native/`; its practice is
`docs/practices/native-bridge.md`. The Electron Bridge, Armada Classic, is
deleted and readable at the tag `classic-final`.

- Pocket builds from `packages/tokens` and its own kit in `pocket/src/kit`.
  Nothing invented.
- Pocket talks to the Phone Gateway Fleet serves on each paired Mac
  (`crates/pocket`), and to nothing else.
- The store keeps claims and pull request owners, one Durable Object per account.

Anything crossing to Rust goes through `docs/practices/protocol.md` first.

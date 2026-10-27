# Spike 34: Can a rebuilt Fleet read a cookie it kept in the Keychain without a prompt?

**Answer: yes, when Fleet reaches the Keychain through `/usr/bin/security` rather than through the Security framework in its own process.**
An item's access list trusts the program that wrote it.
Written by `security`, that program is Apple's signed tool, which does not change when Fleet is rebuilt.
Two builds of the same binary, ad-hoc signed with different CDHashes, both read the item with exit 0 and no dialog.

Measured 19 Oct 2026 on macOS 26.5.2, in a throwaway keychain under `/tmp`, so the person's login keychain and its search list were never touched.
One run.

## What was run

1. `security create-keychain` and `unlock-keychain` on `/tmp/armada-kcspike.keychain-db`.
   The user's search list read the same before and after.
2. A cookie holding a space, a quote, `;` and `=` was written by piping one line to `security -i`: `add-generic-password -U -s <service> -a <host> -X <hex>`.
   **The value travels on stdin as hex**, so it is never in a process's arguments for `ps` to read, and no quoting rule applies to it.
3. A Rust binary that runs `security find-generic-password -s <service> -a <host> -w <keychain>` was built, its version changed, and built again.

| Build | `codesign` | CDHash | Read |
|---|---|---|---|
| 0.1.0 | `adhoc` | `33242a5b…` | exit 0, the cookie byte for byte |
| 0.1.1 | `adhoc` | `9cb066b2…` | exit 0, the cookie byte for byte |

## What it decides

**Fleet keeps a request cookie by running `security`**, one generic-password item per host under one service name, written over stdin as hex and read with `-w`.
**No new dependency is added.**

## What a long cookie showed

Measured 27 Oct 2026, in a throwaway keychain under `/tmp`, after a browser cookie of several kB would not save.
**`security -i` reads 4095 bytes a line, its newline included.**
A 2000-byte value is a 4094-byte line and is kept.
A 2010-byte value is cut: the rest of the line runs as a command, `security: unknown command "…"`, which echoes the hex.
**A cut line loses its keychain argument**, so its first half is written to the default keychain.

So a value is kept in parts of 1500 bytes, the first in the variable's own item and each after it under `armada-request-variable-part`.
A failed write says what `security -i` printed after it, such as `add-generic-password: returned -128` with the Security framework's message.

## What was not measured

- **The Security framework in Fleet's own process**, the `security-framework` crate.
  That would make Fleet's own binary the trusted program, and an ad-hoc build changes its CDHash on every rebuild, so a dialog is the expected result.
  It was not run, because the dialog would have landed on the owner's screen.
- **The login keychain itself.**
  The access list mechanism is the same for any file keychain, and another tool on this machine already reads its own `security`-written login item without a prompt, but this run did not write to the login keychain.

# Themes

**What it is:** The colours Bridge is drawn in: Dark, Light, a catalogue of terminal colour schemes it ships, and the themes mods add.

---

**Kind:** Concept.

## What a theme is

> **Rule.** A theme is one `[data-theme="<id>"]` block that redefines semantic tokens, and nothing else.
> Why: a component reads a token, so a theme re-skins it with no rule of its own and no hex.

| Source | Id | Where the block lives | Loaded |
|---|---|---|---|
| Dark | `dark` | `packages/tokens/src/*.css`, the defaults | With the stylesheet |
| Light | `light` | `packages/tokens/src/light.css` | With the stylesheet |
| Catalogue | `catalogue:<slug>` | `packages/tokens/themes/css/<slug>.css` | When chosen |
| A mod | The mod's name | The mod's `theme.css` | When chosen, and only while the mod is enabled |

**The theme goes on `<html>`.** Every token file that aliases or mixes another declares on `:root, [data-theme]`, so an alias re-resolves on whichever element carries a theme. A themed `<div>`, as a story draws one, is light all the way down.

**Bridge reads a `ThemeSource`** (`packages/surfaces/settings/src/theme-source.tsx`): the mods, the catalogue and the active id, with `setActive`. Settings, the Mods surface and the renderer's loader (`apps/desktop/src/renderer/src/theme.tsx`) read the same source. The mock runs on an in-memory one; Fleet's mod folder replaces it in `main.tsx`.

## What the loader refuses

| Input | Result |
|---|---|
| CSS that is empty, does not parse, has a rule outside `[data-theme="<id>"]`, or contains `@import` | Dark, and the source is told Dark |
| An id that is not offered, or a mod that is off | Dark |
| A catalogue file that cannot be read, or a source that throws | Dark |
| `?nomods` on the page, set by main for `--no-mods` | No mod's theme is loaded or listed; Dark, Light and the catalogue remain |

**The CSS is adopted as a constructable stylesheet, not written as a `<style>` element.** The window's CSP is `default-src 'self'`, which refuses an inline `<style>`; `adoptedStyleSheets` is CSSOM and leaves the policy as it is (`docs/practices/bridge.md`, Security posture).

## The catalogue

**The schemes are Ghostty colour-scheme files**, the collection `mbadolato/iTerm2-Color-Schemes` publishes in its `ghostty/` directory. `packages/tokens/themes/ghostty/` holds the ones Bridge ships, copied at commit `31756e77934045c66b28f59463ddb2870606d7ca`, and `popular.txt` names them.

```
node packages/tokens/themes/generate.mjs            # css/ and index.json from ghostty/ and popular.txt
node packages/tokens/themes/generate.mjs --check    # fails if they are stale or a theme is flagged
node packages/tokens/themes/generate.mjs --all <dir>   # every scheme in <dir> into more/, not checked in
```

`more/` is ignored by git and read by Bridge the same way as `css/`. `<dir>` is the `ghostty/` directory of a sparse checkout of the collection.

**Each theme is loaded when chosen.** `index.json` is read up front, one title and tone per theme; each theme's CSS is a file of its own and a chunk of its own in the build.

### The mapping

A scheme has 16 palette colours, a background and a foreground. Colours are mixed in OKLab; a colour that must read is moved along its own lightness, keeping hue and chroma.

| Bridge token | From |
|---|---|
| `--bg-base` | The background |
| `--bg-sunken`, `--bg-raised`, `--bg-overlay`, `--bg-hover`, `--border-*` | The background mixed toward the foreground; the sunken well toward black in a dark scheme, cards toward white in a light one |
| `--fg-default` | The foreground |
| `--fg-muted`, `--fg-subtle` | The foreground mixed toward the background, as far as 4.5:1 allows |
| `--accent` | The first of palette 12, 4, 6, 14, 5, 13 with colour in it that reads as text |
| `--status-completed-failed`, `-completed-success`, `-awaiting-review`, `-running`, `-rejected` | Palette 1, 2, 3, 4, 5 |
| `--status-escalated` | Palette 1 and 3, half and half |
| `--status-not-started`, `--status-killed` | The foreground mixed toward the background |
| `--helm` | The accent and palette 5, half and half |
| `--tool-look`, `--tool-change`, `--tool-run` | Palette 6, 5, 3 |
| `--diff-*` | Palette 2 and 1 on a tint of the background |

**Dark or light is the background's luminance**, above 0.179, where black and white text tie. A light theme also takes Light's shadows and scrim; a dark one keeps the defaults.

### What is measured

| Pair | Floor |
|---|---|
| `--fg-default`, `--fg-muted`, `--fg-subtle`, `--accent` on each of five grounds | 4.5:1 |
| Each status hue as text on its own 12% tint, on each ground | 4.5:1 |
| Tool colours on the well and on hover | 4.5:1 |
| A scheme's foreground on its own background | 3:1, under which the theme is flagged and not shipped |

A colour moved to pass is listed in its theme's header comment. A flagged theme fails `--check` and `apps/desktop/src/main/themes.test.ts`.

## Where the schemes come from

**The collection is MIT licensed** and the copyright notice is `packages/tokens/themes/LICENSE-iTerm2-Color-Schemes`. Its licence states that the licence for each theme belongs to that theme's author. The collection's `CREDITS.md` names the author of each scheme and does not state a licence for each.

**The themes are listed on cmuxthemes.com**, which names no source and states no licence. Its theme pages print Ghostty configuration, and its names match the collection's.

| Theme | Author, from the collection's credits | Upstream licence |
|---|---|---|
| Catppuccin | The Catppuccin team | MIT |
| Dracula | zenorocha; Dracula+ by jos3s | MIT for Dracula; Dracula+ not found |
| Gruvbox | morhetz | MIT/X11, per the project's README |
| Nord, Nord Light | The Nord project | MIT |
| Rosé Pine | The Rosé Pine project | MIT |
| Tokyo Night | folke | Apache-2.0 |
| Atom One | zasdaym, iamstarkov; after Atom's One | MIT for Atom's One; the ports not found |
| Kanagawa | rebelot | MIT |
| Everforest | sainnhe; ported for iTerm2 by others | MIT |
| Material, Material Ocean | stoeffel; fr3fou after kaicataldo | MIT |
| Nightfox, Carbonfox, Dayfox | EdenEast | MIT |
| Solarized | Ethan Schoonover | MIT |
| GitHub | GitHub, ported by apcamargo | MIT |
| Ayu | alebcay; Mirage from the VS Code theme | MIT |
| Night Owl, Light Owl | sdras; ported by zasdaym and praveenpuglia | MIT |
| Oxocarbon | Nyoom Engineering | MIT |
| Horizon | jolaleye | MIT |
| One Half | sonph | MIT |
| Flexoki | kepano | MIT |

**Not shipped, for want of a licence found:** the Monokai schemes. The rest of the collection is not in the repository; `generate.mjs --all` makes it on a machine that wants it.

## Open questions

- **[catalogue-scheme-licences]** Is a scheme's palette, colours read out of a file, covered by its author's licence at all? The collection's licence leaves each theme's to its author, and for the Dracula+ and Atom One ports no licence was found. What decides it: whether the Atom One and Dracula+ schemes ship on the licence of the theme they port, or leave `popular.txt` until their authors state one.

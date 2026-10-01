# What does ⌘V on an open Studio do?

**Decided 2026-10-01.**

His annotation `20261001-123631-62n3`: *"I would love if when I am focused on a studio, I can press "cmd + v" to paste in whatever is on my clipboard -- a link, a screenshot, a file path, etc"*. After trying the add-in-place option he said *"pasting is not working in the studio canvas"*, because nothing listened for it.

**Chosen:** a paste lands at once as a finished node, sent to Fleet straight away, with no draft and no field opening first.

- Under the pointer where it is over the board, otherwise in the middle of the view.
- An address is a Link. A file path is a File (`2026-10-01-a-pasted-path-is-a-file.md`). A picture is a Picture. Any other text is a Note.
- A paste into a field, a draft's or the Studio's name, is that field's.
- A picture wins over text beside it, unless the picture is only Finder's icon for a copied file. Then it is the file's path.
- Focused on a Studio means the board has focus, and pressing the empty board gives it focus.

**What a real ⌘V hands Chromium on macOS**, measured in Bridge's own Electron against the system clipboard. A sibling's pad paste broke on links and text for him after being tested on made-up events alone, so these are the shapes the tests are built from:

| Copied from | What the paste carries | Path on disk |
|---|---|---|
| An address bar | `text/plain`, the address | — |
| A page's text | `text/plain` and `text/html` | — |
| A terminal | `text/plain`, `text/html` and `text/rtf` | — |
| A file in Finder | One file, by name, with no text and no icon | Yes |
| An image file in Finder | One `image/png` file, by its name | Yes |
| A screenshot | One `image/png` file called `image.png` | No |

Finder's icon never reaches the page. So "only the icon" comes down to whether the file is on disk, which main reads with `webUtils.getPathForFile`. The renderer gets a name and nothing else.

**Cost he took:** a paste offers no line beside an address and no choice to read it in. That offer stays on a Link written in its own field.

**Changed the same day:** a picture is not a Note. Its Note would have had no words, and Fleet refuses a Note with none. He chose a kind of its own, the Picture (`2026-10-01-a-pasted-picture-is-a-picture.md`).

**Where it landed:** `studios/paste`.

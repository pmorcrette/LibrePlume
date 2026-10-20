# LibrePlume for Firefox

Checks what you type in text areas and rich text editors on any website, using
**your own** LibrePlume server. Nothing is sent anywhere else.

## Features

- A badge in the corner of the field you are editing shows how many issues were found
  (✓ when the text is clean); click it to see them and apply fixes with one click, or
  fix everything at once. Fixes go through the browser's editing commands, so
  <kbd>Ctrl</kbd>+<kbd>Z</kbd> still works.
- Inline underlines in rich text editors (Firefox 140+ with the CSS Highlight API).
- Right-click a selection: *Check with LibrePlume* or *Conjugate*.
- Toolbar popup to check any pasted text and copy the corrected version.
- <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>L</kbd> checks the focused field on demand.
- Follows the server's default theme, in French or English.

## Install

During development, open `about:debugging#/runtime/this-firefox`, click
**Load Temporary Add-on…** and pick `manifest.json` in this folder.

To build a package (requires Node.js):

```sh
npx web-ext lint   # static checks
npx web-ext build  # creates web-ext-artifacts/libreplume-*.zip
```

Unsigned packages only install in Firefox Developer Edition/Nightly with
`xpinstall.signatures.required` set to `false`. For regular Firefox, sign the package
on [addons.mozilla.org](https://addons.mozilla.org/developers/) (it can be an unlisted,
self-distributed add-on).

## Configure

Open the add-on options and set the address of your server (default
`http://localhost:8080`). If the server sets `LIBREPLUME_TOKEN`, paste the token to be
able to add words to the shared dictionary. Click **Allow access to websites** if
Firefox has not granted the site permission yet.

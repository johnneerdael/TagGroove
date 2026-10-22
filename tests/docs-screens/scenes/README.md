# Docs screenshot scenes

Each user guide workstream keeps its screenshot scenes in its own file here
(`converters.js`, `options.js`, …), so writers working in parallel never edit
the same file. `scripts/docs-screenshots.sh` loads every `*.js` file in this
folder before `../screens.js`, which defines the core scenes and runs them all
in Light and Dark mode.

A scene file registers a factory. The factory receives the driver's helpers
and returns scenes:

```js
(window.OPENTAG_DOCS_SCENE_FILES = window.OPENTAG_DOCS_SCENE_FILES || []).push(h => [
  { name: 'auto-numbering', run: async () => { await h.selectAll(); h.op('number'); await h.waitFor(() => h.dialog(), 5000, 'wizard'); } },
]);
```

- `name`: kebab-case; the PNGs become `<name>-light.png` and `<name>-dark.png`.
- `run`: brings the app into the state to capture. Before each scene the
  driver closes dialogs and menus and blurs the focus; afterwards it waits,
  then the runner captures the window.

Helpers (`h`): `op(dataOp)` runs a menu or toolbar command by its
`data-op` (see the `Operation` type in `src/main.ts`), `clickRow(i)` selects
row *i* of the file list, `selectAll()`, `menu('Convert')` returns a menu's
`<details>` element (set `.open = true`), `dialog()` returns the open dialog,
`input(el, value)` types into a field, `waitFor(fn, ms, what)`, `sleep(ms)`,
`rows()`, `idle()` and `inv(command, args)` for backend calls.

The album is invented (`../fixtures.sh`): eight MP3s by *Juniper Vale*,
sorted by file name as `01_low_tide`, `02 - Lanterns On The Water`,
`03_paper boats`, `05 - juniper vale - salt & cedar`, `06_the_keeper's_song`,
`07 Fog Horn (Interlude)`, `08 - Harbour Lights`, `track04`. Scenes must not
save changes to the files, because later scenes expect the original album.

## Real-music runs (macOS)

`scripts/docs-screenshots-macos.sh` (owner-approved, opt-in; see `style.md` § 8) runs the same
scene files against a working copy of real albums and songs. It sets
`window.OPENTAG_DOCS_MUSIC` (the copy's root), `OPENTAG_DOCS_FIRST` (the album shown first)
and `OPENTAG_DOCS_LIBRARY` (`{ albums: [...], songs: [...] }`). Extra helpers:

- `openAlbum('albums/<folder>')` replaces the list with that folder or file, so a scene can
  choose material that fits its provider; give a scene `album: 'albums/<folder>'` and the
  driver opens it before `run`. `music('albums', /pattern/)` finds the folder name.
- `selectRows([1, 6])`, `openDialog(op, selector)`, `openMenu('Edit')`, `optionsPage(dialog, id)`.
- `snap(name, crop)` takes an extra image during a scene (list it in `also`); `crop: 'dialog'`
  on a scene captures only the top dialog. Set `noFinalShot: true` when the scene only uses `snap`.
- The driver refuses to capture an error banner (`allowError: true` opts out) and dismisses
  errors between scenes. Scenes must avoid commands that open a native picker or a destructive
  confirmation (Remove tag…, Copy files to…, Open folder).

Scenes that need a live provider (`tag-sources-live.js`) return no scenes unless
`OPENTAG_DOCS_MUSIC` is set, and `TAGGROOVE_DOCS_SKIP="scene-a scene-b"` leaves scenes out when a
provider is down.

## Capture availability

The Linux native job in the reviewed reusable build captures the five core
scenes in light and dark themes after exact-head review authorization. The
former automatic workflow remains removed and its legacy ID stays disabled.
The Linux job and `scripts/docs-screenshots.sh` still refuse to run on a writer's
machine; the macOS runner above is the one exception, for the owner's own Mac.

`scripts/docs-screens-fetch.sh <run-id> [scene …]` downloads `docs-screens`
from an explicit verified run. It does not start capture. Verify the source
revision and inspect every image before committing it; obtain another clean
review of the resulting head.

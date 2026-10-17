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

## Capture availability

The Linux native job in the reviewed reusable build captures the five core
scenes in light and dark themes after exact-head review authorization. The
former automatic workflow remains removed and its legacy ID stays disabled.

Add proposed scenes to the appropriate workstream file and the reviewed
job's explicit scene list. Pushing a scene alone does not execute it. Never
run the screenshot script or the app on a writer's machine for guide work.

`scripts/docs-screens-fetch.sh <run-id> [scene …]` downloads `docs-screens`
from an explicit verified run. It does not start capture. Verify the source
revision and inspect every image before committing it; obtain another clean
review of the resulting head. Historical artifacts can still be reused when
their source and images match the page, provided they have not expired.

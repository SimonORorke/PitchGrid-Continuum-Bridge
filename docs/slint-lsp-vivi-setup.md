# Slint LSP and Live-Preview: Setting Up @vivi Imports

## Background

This project uses [vivi](https://radicle.network/nodes/seed.radicle.garden/rad%3Az3oxAZSLcyXgpa7fcvgtueF49jHpH),
a custom Slint component library. Slint files import from it using the `@vivi` library prefix, for example:

```slint
import { MagicText } from "@vivi/magic.slint";
import { TextStyle } from "@vivi/foundation.slint";
```

At **compile time** this works automatically: `build.rs` calls `vivi_ui::import_paths()`, which
tells the Slint compiler where to find `@vivi` files in the Cargo registry.

The **Slint LSP** (used by the RustRover IDE plugin and Slint Live-Preview) does not go through the
Rust build system, so it has no knowledge of the `@vivi` library path. Without extra setup it
reports errors such as:

```
Cannot find requested import "@vivi/magic.slint" in the library search path
```

and Slint Live-Preview shows "Preview is out of date".

## Fix: Windows NTFS Directory Junctions

The Slint LSP resolves `@vivi/...` imports by looking for a subdirectory literally named `@vivi`
relative to the directory of the importing `.slint` file. Creating Windows NTFS *directory
junctions* (transparent filesystem redirects) at those locations makes the LSP follow them to the
actual vivi library files in the Cargo registry.

> **Important:** The junctions must be proper Windows NTFS junctions, **not** Git Bash / MSYS2
> symlinks (created with `ln -s`). Git Bash symlinks are invisible to native Windows applications
> like the Slint LSP. Node.js's `fs.symlinkSync(target, link, 'junction')` creates a true NTFS
> junction.

Two junctions are needed because `.slint` files that import `@vivi` exist in two directories:

| Junction location | Files it serves |
|---|---|
| `ui/@vivi` | `main_window.slint`, `styling.slint`, `about_window.slint` |
| `ui/components/@vivi` | `text.slint`, `midi_io.slint` |

Both junctions point to the same target: the `ui/` directory inside the `vivi_ui` crate in the
Cargo registry.

## Creating the Junctions

A Node.js script, `make_junction.js` in the project root, creates both junctions automatically.
Run it once after cloning the repository:

```
node make_junction.js
```

Node.js is available from the JetBrains runtime bundled with RustRover, or can be installed
separately from [nodejs.org](https://nodejs.org).

The script reads the `vivi_ui` version from the Cargo registry path automatically via
`os.homedir()`, so no manual path editing is needed unless the registry index directory name
changes (which it does not in practice).

After running the script, **restart** RustRover (or reload the Slint LSP) so the plugin picks up
the new directory structure.

## What the Script Does

```javascript
// make_junction.js (project root)
const fs   = require('fs');
const os   = require('os');
const path = require('path');

const viviTarget = path.join(
    os.homedir(), '.cargo', 'registry', 'src',
    'index.crates.io-1949cf8c6b5b557f', 'vivi_ui-0.2.0', 'ui'
);
const projectUi = path.join(__dirname, 'ui');  // always relative to the script's location
const links = [
    path.join(projectUi, '@vivi'),
    path.join(projectUi, 'components', '@vivi'),
];

for (const link of links) {
    if (fs.existsSync(link)) fs.rmSync(link, { recursive: true });
    fs.symlinkSync(viviTarget, link, 'junction');
    console.log('Junction created:', link);
}
```

> The script uses `__dirname` to locate the `ui/` directory, so it works correctly regardless of
> where the repository is checked out.

## Git

The two junction directories are listed in `.gitignore` so they are never committed:

```
/ui/@vivi
/ui/components/@vivi
```

The script `make_junction.js` **is** committed to the repository so collaborators can run it.

## Upgrading vivi_ui

If `vivi_ui` is upgraded to a new version, update the version number in `make_junction.js`
and re-run it:

1. Open `make_junction.js`.
2. Change `vivi_ui-0.2.0` to the new version, e.g. `vivi_ui-0.3.0`.
3. Run `node make_junction.js`.

## Slint Live-Preview: Additional Requirement

Slint Live-Preview (in the RustRover Slint plugin) only has access to files that are currently
**open in editor tabs**. To preview a component in `main_window.slint`, all files it imports
must be open simultaneously:

- `ui/globals.slint`
- `ui/styling.slint`
- `ui/about_window.slint`
- `ui/components/text.slint`
- `ui/components/midi_io.slint`
- `ui/components/nice_menu.slint`

Open them all in tabs, then trigger a refresh in the Live-Preview panel.

## styling.slint

`ui/styling.slint` is a copy of `vivi_ui/ui/magic/styling.slint` from the local vivi development
source. The published crate (`vivi_ui-0.2.0` on crates.io) ships a different version of that file
with an incompatible API, so the project carries its own copy. Once a version of vivi is published
whose `magic/styling.slint` matches the API used here (exporting `Catppuccin`, `Flavor`,
`CatppuccinPalette`, `Palette`, `FontSettings`, etc.), the local copy can be deleted and replaced
with a direct import from `@vivi/magic/styling.slint`.

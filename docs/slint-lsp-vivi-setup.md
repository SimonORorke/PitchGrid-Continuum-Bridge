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

The script discovers everything it needs automatically, so there is **no manual path or version
editing**:

- The `vivi_ui` **version** is read from `Cargo.lock` (the resolved dependency).
- The **registry path** is built from `os.homedir()` and globbed across whatever registry index
  directory exists, so it works regardless of the Windows user profile or the registry index hash
  (sparse vs. git protocol).

If the script can't find the crate it prints a clear message; the usual cause is that `cargo build`
has not been run yet, so Cargo hasn't extracted the `vivi_ui` sources into the registry cache.

After running the script, **restart** RustRover (or reload the Slint LSP) so the plugin picks up
the new directory structure.

## What the Script Does

The script does three things; see `make_junction.js` for the full source. In outline:

```javascript
// 1. Read the resolved vivi_ui version from Cargo.lock.
const version = /* parse the [[package]] block for name = "vivi_ui" */;

// 2. Glob the Cargo cache for the matching extracted sources, under whatever
//    registry index dir exists (handles any hash / Windows profile).
const srcRoot = path.join(os.homedir(), '.cargo', 'registry', 'src');
const viviTarget = /* first <index>/vivi_ui-<version>/ui that exists, else error */;

// 3. (Re)create the two junctions, robustly removing any existing ones first.
const links = [path.join(__dirname, 'ui', '@vivi'),
               path.join(__dirname, 'ui', 'components', '@vivi')];
for (const link of links) {
    try {
        fs.lstatSync(link);                              // lstat: detects a *dangling* junction too
        try { fs.chmodSync(link, 0o666); } catch (e) {}  // clear READONLY so removal can't be denied
        fs.rmSync(link, { recursive: true, force: true });
    } catch (e) { if (e.code !== 'ENOENT') throw e; }
    fs.symlinkSync(viviTarget, link, 'junction');
}
```

> The script uses `__dirname` to locate `Cargo.lock` and the `ui/` directory, so it works regardless
> of where the repository is checked out, and matching the version against `Cargo.lock` means an
> upgrade needs no edit to this script.

> **Two non-obvious failure modes the removal logic guards against** (both hit in practice after the
> machine's C: drive / Windows profile was replaced):
>
> 1. **Dangling junction.** If a junction's target no longer exists (e.g. the profile changed from
>    `C:\Users\Simon O'Rorke` to `C:\Users\User`), `fs.existsSync(link)` *follows* the junction and
>    returns `false`, so the stale junction is never removed and `symlinkSync` then throws `EEXIST`.
>    Re-running the script appears to do nothing. Use `fs.lstatSync` (does not follow) instead.
> 2. **READONLY junction.** A junction carrying the READONLY attribute cannot be removed — `rmdir` /
>    `fs.rmSync` fail with "Access is denied", which mimics a process lock (killing the LSP / file
>    watcher does not help). Clear the readonly bit (`fs.chmodSync(link, 0o666)`) before removing.
>    Note: ordinary directories ignore the readonly bit for deletion; only reparse points are blocked
>    by it. The origin of the flag on one of the junctions here is unknown (it was *not* confirmed to
>    be OneDrive — directory-readonly is a normal Windows flag), but clearing it before removal is a
>    safe, cheap guard regardless.

## Git

The two junction directories are listed in `.gitignore` so they are never committed:

```
/ui/@vivi
/ui/components/@vivi
```

The script `make_junction.js` **is** committed to the repository so collaborators can run it.

## Upgrading vivi_ui

If `vivi_ui` is upgraded to a new version, no edit to `make_junction.js` is needed — it reads the
version from `Cargo.lock`. After the dependency is bumped and the project is built:

1. Run `cargo build` (so Cargo extracts the new `vivi_ui` sources into the registry cache).
2. Run `node make_junction.js` (it picks up the new version automatically and re-points the junctions).
3. Restart RustRover or reload the Slint LSP.

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

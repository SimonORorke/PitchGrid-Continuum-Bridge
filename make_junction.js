// Creates the Windows NTFS directory junctions that let the Slint LSP / Live-Preview
// resolve `@vivi/...` imports. For the full rationale, see docs\slint-lsp-vivi-setup.md.
//
// Both the vivi_ui version and the Cargo registry path are discovered automatically:
//   - version: read from Cargo.lock (the resolved dependency), so an upgrade needs no edit here.
//   - registry: globbed from the Cargo cache, so it works regardless of the registry index hash
//     (sparse vs. git protocol) or the Windows user profile.
const fs = require('fs');
const os = require('os');
const path = require('path');

// 1. Resolve the locked vivi_ui version from Cargo.lock.
const lockPath = path.join(__dirname, 'Cargo.lock');
let version = null;
try {
    const lock = fs.readFileSync(lockPath, 'utf8');
    for (const block of lock.split('[[package]]')) {
        const name = /name\s*=\s*"([^"]+)"/.exec(block);
        if (name && name[1] === 'vivi_ui') {
            const ver = /version\s*=\s*"([^"]+)"/.exec(block);
            if (ver) version = ver[1];
            break;
        }
    }
} catch (e) {
    console.error('Could not read Cargo.lock at', lockPath, ':', e.message);
    process.exit(1);
}
if (!version) {
    console.error('vivi_ui not found in Cargo.lock — is it still a dependency?');
    process.exit(1);
}

// 2. Locate the extracted crate sources under any registry index in the Cargo cache.
//    Matching the locked version (not just the highest folder present) avoids picking a
//    stale cached version that lingers in the registry.
const srcRoot = path.join(os.homedir(), '.cargo', 'registry', 'src');
let viviTarget = null;
try {
    for (const indexDir of fs.readdirSync(srcRoot)) {
        const candidate = path.join(srcRoot, indexDir, `vivi_ui-${version}`, 'ui');
        if (fs.existsSync(candidate)) { viviTarget = candidate; break; }
    }
} catch (e) {
    console.error('Could not read the Cargo registry at', srcRoot, ':', e.message);
    process.exit(1);
}
if (!viviTarget) {
    console.error(`vivi_ui-${version}/ui not found under ${srcRoot}.`);
    console.error('Run "cargo build" first so Cargo extracts the crate sources, then re-run.');
    process.exit(1);
}
console.log(`vivi_ui ${version} -> ${viviTarget}`);

// 3. Create the junctions.
const projectUi = path.join(__dirname, 'ui');  // always relative to the script's location
const links = [
    path.join(projectUi, '@vivi'),
    path.join(projectUi, 'components', '@vivi'),
];

for (const link of links) {
    try {
        // lstatSync (not existsSync) so a *dangling* junction — one whose target no longer
        // exists, e.g. after the Windows user profile changes — is still detected and removed.
        // existsSync follows the link and reports false for a dangling junction, which leaves
        // the stale entry in place and makes symlinkSync throw EEXIST.
        try {
            fs.lstatSync(link);
            // A READONLY junction cannot be removed (rmdir/rmSync fail with "Access is denied"),
            // which mimics a process lock. Clear the readonly bit on the junction itself first.
            try { fs.chmodSync(link, 0o666); } catch (e) { /* best effort */ }
            fs.rmSync(link, { recursive: true, force: true });
            console.log('Removed existing:', link);
        } catch (e) {
            if (e.code !== 'ENOENT') throw e;
        }
        fs.symlinkSync(viviTarget, link, 'junction');
        console.log('Junction created:', link);
    } catch (e) {
        console.error('Error for', link, ':', e.message);
    }
}

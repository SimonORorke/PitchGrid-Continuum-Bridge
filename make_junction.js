// For the purpose and usage of this nodejs script, see docs\slint-lsp-vivi-setup.md.
const fs = require('fs');
const os = require('os');
const path = require('path');

const viviTarget = path.join(os.homedir(), '.cargo', 'registry', 'src', 'index.crates.io-1949cf8c6b5b557f', 'vivi_ui-0.2.0', 'ui');
const projectUi = path.join(__dirname, 'ui');

const links = [
    path.join(projectUi, '@vivi'),
    path.join(projectUi, 'components', '@vivi'),
];

for (const link of links) {
    try {
        if (fs.existsSync(link)) {
            fs.rmSync(link, { recursive: true });
            console.log('Removed existing:', link);
        }
        fs.symlinkSync(viviTarget, link, 'junction');
        console.log('Junction created:', link);
    } catch (e) {
        console.error('Error for', link, ':', e.message);
    }
}

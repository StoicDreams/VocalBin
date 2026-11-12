import { test, ROOT_FILES } from '../tests/support/site.mjs';
import { join } from 'node:path';

// Renders Vocal Bin's brand images from webapp/root_files/Logo.svg, so they always match the logo and palette:
// the app icons (apple-icon.png, ms-icon-512x512.png) and the 1200x630 image shared links show (og:image).
// Run it with PowerShell/screenshots.ps1, or npm run screenshots, after changing the logo or the palette.

const BACKGROUND = '#333333';
const TITLE = '#d10694';

test.describe.configure({ mode: 'serial' });

for (const [file, size] of [['apple-icon.png', 192], ['ms-icon-512x512.png', 512]]) {
    test(`app icon ${file}`, async ({ page }) => {
        await page.setViewportSize({ width: size, height: size });
        await page.goto('/');
        await page.setContent(`<!doctype html><html><body style="margin:0;background:${BACKGROUND}">
<img src="/Logo.svg" alt="" style="display:block;width:${size}px;height:${size}px"></body></html>`);
        await page.locator('img').evaluate(img => img.decode());
        await page.screenshot({ path: join(ROOT_FILES, file) });
    });
}

test('share image for links', async ({ page }) => {
    await page.setViewportSize({ width: 1200, height: 630 });
    await page.goto('/');
    await page.setContent(`<!doctype html><html><body style="margin:0">
<div style="width:1200px;height:630px;box-sizing:border-box;display:flex;align-items:center;gap:72px;padding:0 96px;
background:${BACKGROUND};border-bottom:24px solid ${TITLE};font-family:'Noto Sans','DejaVu Sans',Arial,sans-serif;color:#fff">
<img src="/Logo.svg" alt="" style="width:340px;height:340px;flex:none">
<div>
<div style="font-size:96px;font-weight:800;line-height:1.05">Vocal Bin</div>
<div style="font-size:44px;margin-top:20px;color:${TITLE};font-weight:700">Text to speech, built from scratch</div>
<div style="font-size:30px;margin-top:28px;opacity:0.85">No recordings. No AI. Every sound made in your browser.</div>
</div>
</div></body></html>`);
    await page.locator('img').evaluate(img => img.decode());
    await page.screenshot({ path: join(ROOT_FILES, 'img', 'vocal-bin-share.png') });
});

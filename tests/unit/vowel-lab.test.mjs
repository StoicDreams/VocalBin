import { test, expect, openPage } from './support/site.mjs';

// app-vowel-lab: stage 1 of the speech synthesizer (synth/, built into webapp/root_files/vocal-synth.wasm).
const lab = page => page.locator('app-vowel-lab');
const slider = (page, name) => lab(page).getByRole('slider', { name });
const shown = (page, name) => slider(page, name).locator('xpath=preceding-sibling::output');

test.beforeEach(async ({ page }) => {
    await openPage(page, '/vowel-lab');
});

test('shows every vowel, voice, and slider', async ({ page }) => {
    await expect(lab(page).locator('[data-vowel]')).toHaveCount(12);
    await expect(lab(page).getByRole('button', { name: 'ɑ as in hod' })).toHaveAttribute('aria-pressed', 'true');
    for (const name of ['Man', 'Woman', 'Child', 'Play', 'Record']) {
        await expect(lab(page).getByRole('button', { name, exact: true })).toBeVisible();
    }
    await expect(lab(page).getByRole('slider')).toHaveCount(8);
    await expect(shown(page, 'Pitch')).toHaveText('110 Hz');
});

test('sliders show their values to sight and to screen readers', async ({ page }) => {
    await slider(page, 'Pitch').fill('200');
    await expect(shown(page, 'Pitch')).toHaveText('200 Hz');
    await expect(slider(page, 'Pitch')).toHaveAttribute('aria-valuetext', '200 Hz');
    await slider(page, 'Voice quality').fill('2.5');
    await expect(shown(page, 'Voice quality')).toHaveText('lax (2.50)');
});

test('voice presets move the sliders', async ({ page }) => {
    await lab(page).getByRole('button', { name: 'Woman' }).click();
    await expect(shown(page, 'Pitch')).toHaveText('210 Hz');
    await expect(shown(page, 'Vocal tract length')).toHaveText('14.8 cm');
    await lab(page).getByRole('button', { name: 'Child' }).click();
    await expect(shown(page, 'Pitch')).toHaveText('290 Hz');
});

test('marks the chosen vowel', async ({ page }) => {
    await lab(page).getByRole('button', { name: 'i as in heed' }).click();
    await expect(lab(page).getByRole('button', { name: 'i as in heed' })).toHaveAttribute('aria-pressed', 'true');
    await expect(lab(page).locator('[aria-pressed="true"][data-vowel]')).toHaveCount(1);
});


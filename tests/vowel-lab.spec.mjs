import { test, expect, openPage } from './support/site.mjs';
import { readFileSync } from 'node:fs';

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

test('play toggles between Play and Stop', async ({ page }) => {
    const play = lab(page).locator('.play');
    await play.click();
    await expect(play).toHaveText('Stop');
    await expect(play).toHaveAttribute('aria-pressed', 'true');
    await play.click();
    await expect(play).toHaveText('Play');
    await expect(play).toHaveAttribute('aria-pressed', 'false');
});

test('records what is played as a WAV file', async ({ page }) => {
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await lab(page).locator('.play').click();
    await lab(page).getByRole('button', { name: 'Record' }).click();
    await expect(lab(page).getByRole('status')).toHaveText(/Recording 0:01/, { timeout: 10_000 });
    const download = page.waitForEvent('download');
    await lab(page).getByRole('button', { name: 'Stop recording' }).click();
    const file = readFileSync(await (await download).path());

    expect(file.toString('ascii', 0, 4)).toBe('RIFF');
    expect(file.toString('ascii', 8, 12)).toBe('WAVE');
    const sampleRate = file.readUInt32LE(24);
    expect(sampleRate).toBeGreaterThanOrEqual(8000);
    const samples = (file.length - 44) / 2;
    expect(samples / sampleRate).toBeGreaterThan(0.9);
    let sum = 0;
    for (let i = 44; i < file.length; i += 2) sum += (file.readInt16LE(i) / 32768) ** 2;
    expect(Math.sqrt(sum / samples), 'the recording should hold the voice, not silence').toBeGreaterThan(0.02);

    await expect(lab(page).getByRole('status')).toHaveText(/^Saved \d+:\d\d as vocal-bin-vowel-lab-.*\.wav$/);
    await expect(lab(page).getByRole('button', { name: 'Record' })).toBeEnabled();
    expect(errors).toEqual([]);
});

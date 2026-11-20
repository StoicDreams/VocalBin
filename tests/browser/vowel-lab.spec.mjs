import { test, expect, openPage } from '../support/site.mjs';
import { readFileSync } from 'node:fs';

// app-vowel-lab: stage 1 of the speech synthesizer (synth/, built into webapp/root_files/vocal-synth.wasm).
const lab = page => page.locator('app-vowel-lab');

test.beforeEach(async ({ page }) => {
    await openPage(page, '/vowel-lab');
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

test('play toggles between Play and Stop', async ({ page }) => {
    const play = lab(page).locator('.play');
    await play.click();
    await expect(play).toHaveText('Stop');
    await expect(play).toHaveAttribute('aria-pressed', 'true');
    await play.click();
    await expect(play).toHaveText('Play');
    await expect(play).toHaveAttribute('aria-pressed', 'false');
});

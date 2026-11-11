import { test, expect, openPage } from './support/site.mjs';
import { readFileSync } from 'node:fs';

// app-phoneme-lab: stage 2 of the speech synthesizer (synth/, built into webapp/root_files/vocal-synth.wasm).
const lab = page => page.locator('app-phoneme-lab');
const text = page => lab(page).getByRole('textbox', { name: 'Phonemes (ARPAbet)' });
const status = page => lab(page).getByRole('status');
const slider = (page, name) => lab(page).getByRole('slider', { name });

test.beforeEach(async ({ page }) => {
    await openPage(page, '/phoneme-lab');
});

/** Saves the current text as a WAV, and returns its sample rate and samples. */
async function saveWav(page) {
    const download = page.waitForEvent('download');
    await lab(page).getByRole('button', { name: 'Save WAV' }).click();
    const file = readFileSync(await (await download).path());
    expect(file.toString('ascii', 0, 4)).toBe('RIFF');
    expect(file.toString('ascii', 8, 12)).toBe('WAVE');
    const samples = [];
    for (let i = 44; i < file.length; i += 2) samples.push(file.readInt16LE(i) / 32768);
    return { sampleRate: file.readUInt32LE(24), samples };
}

test('starts with an example, the controls, and a phoneme chart', async ({ page }) => {
    await expect(text(page)).toHaveValue('HH AH0 L OW1 , W ER1 L D .');
    for (const name of ['Speak', 'Save WAV', 'Man', 'Woman', 'Child', 'How are you?']) {
        await expect(lab(page).getByRole('button', { name, exact: true })).toBeVisible();
    }
    await expect(lab(page).getByRole('slider')).toHaveCount(8);
    await lab(page).getByText('Phoneme chart').click();
    await expect(lab(page).locator('[data-symbol]')).toHaveCount(39);
    await expect(lab(page).getByRole('button', { name: 'AY, aɪ as in hide' })).toBeVisible();
});

test('the chart adds phonemes at the cursor', async ({ page }) => {
    await text(page).fill('HH');
    await lab(page).getByText('Phoneme chart').click();
    await lab(page).getByRole('button', { name: 'AY, aɪ as in hide' }).click();
    await lab(page).getByRole('button', { name: 'Z, z as in zee' }).click();
    await expect(text(page)).toHaveValue('HH AY1 Z ');
});

test('examples replace the text', async ({ page }) => {
    await lab(page).getByRole('button', { name: 'How are you?' }).click();
    await expect(text(page)).toHaveValue('HH AW1 AA1 R Y UW1 ?');
});

test('points out a phoneme it does not know', async ({ page }) => {
    await text(page).fill('HH AH0 L XX1');
    await lab(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText(/“XX1” isn't an ARPAbet phoneme/);
    await expect(text(page)).toHaveAttribute('aria-invalid', 'true');
    await text(page).press('End');
    await text(page).pressSequentially(' ');
    await expect(text(page)).not.toHaveAttribute('aria-invalid');
});

test('asks for text when there is none', async ({ page }) => {
    await text(page).fill('   ');
    await lab(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText('Type some phonemes first, or pick an example.');
});

test('saves the speech as a WAV file and shows its timeline', async ({ page }) => {
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const { sampleRate, samples } = await saveWav(page);
    expect(sampleRate).toBe(44100);
    expect(samples.length / sampleRate).toBeGreaterThan(1);
    expect(samples.length / sampleRate).toBeLessThan(3);
    const rms = Math.sqrt(samples.reduce((sum, s) => sum + s * s, 0) / samples.length);
    expect(rms, 'the recording should hold speech, not silence').toBeGreaterThan(0.02);
    await expect(status(page)).toHaveText(/^Saved \d+\.\d seconds as vocal-bin-phoneme-lab-.*\.wav$/);
    await expect(lab(page).locator('.timeline span')).toHaveCount(10);
    await expect(lab(page).locator('.timeline span').nth(1)).toHaveText('AH0');
    expect(errors).toEqual([]);
});

test('speaking faster makes a shorter recording', async ({ page }) => {
    const normal = await saveWav(page);
    await slider(page, 'Speed').fill('2');
    await expect(slider(page, 'Speed')).toHaveAttribute('aria-valuetext', '2.00×');
    const fast = await saveWav(page);
    const ratio = fast.samples.length / normal.samples.length;
    expect(ratio).toBeGreaterThan(0.45);
    expect(ratio).toBeLessThan(0.75);
});

test('every phoneme in the chart can be spoken', async ({ page }) => {
    await text(page).fill('');
    await lab(page).getByText('Phoneme chart').click();
    for (const button of await lab(page).locator('[data-symbol]').all()) {
        await button.click();
    }
    const { samples } = await saveWav(page);
    expect(samples.length).toBeGreaterThan(44100);
});

test('speak plays, then stops by itself or when asked', async ({ page }) => {
    const speak = lab(page).locator('.speak');
    await lab(page).getByRole('button', { name: 'Woman' }).click();
    await speak.click();
    await expect(speak).toHaveText('Stop');
    await expect(speak).toHaveAttribute('aria-pressed', 'true');
    await expect(lab(page).locator('.timeline span.now')).toHaveCount(1);
    await speak.click();
    await expect(speak).toHaveText('Speak');
    await expect(lab(page).locator('.timeline span.now')).toHaveCount(0);

    await text(page).fill('B IY1 .');
    await speak.click();
    await expect(speak).toHaveText('Stop');
    await expect(speak).toHaveText('Speak', { timeout: 5000 });
});

test('Ctrl+Enter speaks', async ({ page }) => {
    await text(page).press('Control+Enter');
    await expect(lab(page).locator('.speak')).toHaveText('Stop');
});

test('loop repeats the speech and picks up changes on the next pass', async ({ page }) => {
    const speak = lab(page).locator('.speak');
    const loop = lab(page).getByRole('button', { name: 'Loop' });
    await expect(loop).toHaveAttribute('aria-pressed', 'false');
    await loop.click();
    await expect(loop).toHaveAttribute('aria-pressed', 'true');

    await text(page).fill('B IY1 .');
    await speak.click();
    await expect(status(page)).toHaveText(/^2 phonemes, (\d+\.\d) seconds\.$/);
    const first = await status(page).textContent();
    // The clip is well under a second, so it is still playing only because it loops.
    await page.waitForTimeout(2500);
    await expect(speak).toHaveText('Stop');

    await slider(page, 'Speed').fill('0.5');
    await expect(status(page)).not.toHaveText(first, { timeout: 5000 });
    await expect(status(page)).toHaveText(/^2 phonemes, \d+\.\d seconds\.$/);

    await speak.click();
    await expect(speak).toHaveText('Speak');
    await expect(speak).toHaveAttribute('aria-pressed', 'false');
});

test('turning loop off lets the current pass finish', async ({ page }) => {
    const speak = lab(page).locator('.speak');
    await lab(page).getByRole('button', { name: 'Loop' }).click();
    await text(page).fill('B IY1 .');
    await speak.click();
    await expect(speak).toHaveText('Stop');
    await lab(page).getByRole('button', { name: 'Loop' }).click();
    await expect(speak).toHaveText('Speak', { timeout: 5000 });
});

import { test, expect, openPage } from '../support/site.mjs';
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

test('points out a phoneme it does not know', async ({ page }) => {
    await text(page).fill('HH AH0 L XX1');
    await lab(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText(/“XX1” isn't an ARPAbet phoneme/);
    await expect(text(page)).toHaveAttribute('aria-invalid', 'true');
    await text(page).press('End');
    await text(page).pressSequentially(' ');
    await expect(text(page)).not.toHaveAttribute('aria-invalid');
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
    // Renders all 39 phonemes into one WAV, which can take longer than the usual timeout on a slow or busy machine.
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
    // The button says Stop while the speech renders, and a phoneme is only highlighted once the audio plays.
    await expect(lab(page).locator('.timeline span.now')).toHaveCount(1, { timeout: 15000 });
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
    // The player sends a reading each time a pass is ready to play, so count the passes.
    await lab(page).locator('app-speech-player').evaluate(player => {
        window.passes = 0;
        player.addEventListener('reading', () => window.passes++);
    });
    await speak.click();
    await expect(status(page)).toHaveText(/^2 phonemes, (\d+\.\d) seconds\.$/);
    const first = await status(page).textContent();
    // The clip plays once without Loop, so a third pass shows it is repeating.
    await expect.poll(() => page.evaluate(() => window.passes)).toBeGreaterThanOrEqual(3);
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

const ipaText = page => lab(page).getByRole('textbox', { name: 'Phonemes (IPA)' });
const notation = (page, name) => lab(page).getByRole('group', { name: 'Notation' }).getByRole('button', { name });

test('IPA examples sound the same as their ARPAbet', async ({ page }) => {
    for (const example of ['Hello, world.', 'She sells sea shells.']) {
        await notation(page, 'ARPAbet').click();
        await lab(page).getByRole('button', { name: example }).click();
        const arpabet = await saveWav(page);
        await notation(page, 'IPA').click();
        await lab(page).getByRole('button', { name: example }).click();
        const ipa = await saveWav(page);
        expect(ipa.samples.length, example).toBe(arpabet.samples.length);
        expect(ipa.samples.every((s, i) => s === arpabet.samples[i]), example).toBe(true);
    }
    await expect(lab(page).locator('.timeline span').first()).toHaveText('ʃ');
});

test('IPA points out a sound it does not know', async ({ page }) => {
    await notation(page, 'IPA').click();
    await ipaText(page).fill('ˈbʌʔn');
    await lab(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText(/^“ʔ” isn't a sound this English voice knows/);
    await expect(ipaText(page)).toHaveAttribute('aria-invalid', 'true');
});

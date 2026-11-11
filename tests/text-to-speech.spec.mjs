import { test, expect, openPage } from './support/site.mjs';
import { readFileSync } from 'node:fs';

// app-text-to-speech: stage 3 of the speech synthesizer, reading English with the pronouncing dictionary
// (webapp/root_files/vocal-synth-en-us.dict) and synth/'s English reader.
const tts = page => page.locator('app-text-to-speech');
const text = page => tts(page).getByRole('textbox', { name: 'Text' });
const status = page => tts(page).getByRole('status');
const ipa = page => tts(page).locator('.ipa');

test.beforeEach(async ({ page }) => {
    await openPage(page, '/text-to-speech');
});

/** Saves the current text as a WAV, and returns its sample rate and samples. */
async function saveWav(page) {
    const download = page.waitForEvent('download');
    await tts(page).getByRole('button', { name: 'Save WAV' }).click();
    const file = readFileSync(await (await download).path());
    expect(file.toString('ascii', 0, 4)).toBe('RIFF');
    expect(file.toString('ascii', 8, 12)).toBe('WAVE');
    const samples = [];
    for (let i = 44; i < file.length; i += 2) samples.push(file.readInt16LE(i) / 32768);
    return { sampleRate: file.readUInt32LE(24), samples };
}

test('starts with an example and the controls', async ({ page }) => {
    await expect(text(page)).toHaveValue('Hello! Welcome to Vocal Bin. How are you today?');
    for (const name of ['Speak', 'Save WAV', 'Man', 'Woman', 'Child', 'Numbers', 'Questions']) {
        await expect(tts(page).getByRole('button', { name, exact: true })).toBeVisible();
    }
    await expect(tts(page).getByRole('slider')).toHaveCount(8);
});

test('saves the speech as a WAV file, with the words and their pronunciation', async ({ page }) => {
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const { sampleRate, samples } = await saveWav(page);
    expect(sampleRate).toBe(44100);
    expect(samples.length / sampleRate).toBeGreaterThan(2);
    expect(samples.length / sampleRate).toBeLessThan(8);
    const rms = Math.sqrt(samples.reduce((sum, s) => sum + s * s, 0) / samples.length);
    expect(rms, 'the recording should hold speech, not silence').toBeGreaterThan(0.02);
    await expect(status(page)).toHaveText(/^Saved \d+\.\d seconds as vocal-bin-speech-.*\.wav$/);
    await expect(tts(page).locator('.reading span')).toHaveCount(9);
    await expect(tts(page).locator('.reading span').first()).toHaveText('Hello');
    await expect(ipa(page)).toHaveText(/^həˈloʊ! ˈwɛlkəm tə ˈvoʊkəl ˈbɪn\. ˈhaʊ ɚ ju təˈdeɪ\?$/);
    expect(errors).toEqual([]);
});

test('reads words from the dictionary, and numbers as words', async ({ page }) => {
    await text(page).fill('The colonel paid $5 on May 5.');
    await saveWav(page);
    // Spelling rules alone couldn't say "colonel".
    await expect(ipa(page)).toHaveText('ðə ˈkɝnəl ˈpeɪd ˈfaɪv ˈdɑlɚz ɑn ˈmeɪ ˈfɪfθ.');
    await expect(status(page)).toHaveText(/^Saved/);
});

test('examples replace the text', async ({ page }) => {
    await tts(page).getByRole('button', { name: 'Questions' }).click();
    await expect(text(page)).toHaveValue('Is it raining? Where did you leave the umbrella?');
});

test('asks for text when there is none', async ({ page }) => {
    await text(page).fill('   ');
    await tts(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText('Type some text first, or pick an example.');
});

test('points out text that is too long to say at once', async ({ page }) => {
    await text(page).fill('word '.repeat(1000));
    await tts(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText(/more than we can say in one go/);
    await expect(text(page)).toHaveAttribute('aria-invalid', 'true');
    await text(page).fill('word');
    await expect(text(page)).not.toHaveAttribute('aria-invalid');
});

test('speak plays and highlights each word, then stops by itself or when asked', async ({ page }) => {
    const speak = tts(page).locator('.speak');
    await tts(page).getByRole('button', { name: 'Woman' }).click();
    await speak.click();
    await expect(speak).toHaveText('Stop');
    await expect(speak).toHaveAttribute('aria-pressed', 'true');
    await expect(tts(page).locator('.reading span.now')).toHaveCount(1);
    await speak.click();
    await expect(speak).toHaveText('Speak');
    await expect(tts(page).locator('.reading span.now')).toHaveCount(0);

    await text(page).fill('Hi.');
    await speak.click();
    await expect(speak).toHaveText('Stop');
    await expect(speak).toHaveText('Speak', { timeout: 5000 });
});

test('Ctrl+Enter speaks', async ({ page }) => {
    await text(page).press('Control+Enter');
    await expect(tts(page).locator('.speak')).toHaveText('Stop');
});

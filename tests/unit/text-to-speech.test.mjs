import { test, expect, openPage } from './support/site.mjs';

// app-text-to-speech: stage 3 of the speech synthesizer, reading English with the pronouncing dictionary
// (webapp/root_files/vocal-synth-en-us.dict) and synth/'s English reader.
const tts = page => page.locator('app-text-to-speech');
const text = page => tts(page).getByRole('textbox', { name: 'Text' });
const status = page => tts(page).getByRole('status');

test.beforeEach(async ({ page }) => {
    await openPage(page, '/text-to-speech');
});

test('starts with an example and the controls', async ({ page }) => {
    await expect(text(page)).toHaveValue('Hello! Welcome to Vocal Bin. How are you today?');
    for (const name of ['Speak', 'Save WAV', 'Man', 'Woman', 'Child', 'Numbers', 'Questions']) {
        await expect(tts(page).getByRole('button', { name, exact: true })).toBeVisible();
    }
    await expect(tts(page).getByRole('slider')).toHaveCount(8);
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


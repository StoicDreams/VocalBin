import { test, expect, openPage } from './support/site.mjs';

// app-speech-player: Speak, Loop, and Save WAV for the Phoneme Lab and Text to Speech. It splits long text into parts
// and starts speaking when the first part is ready.
const player = (page, host) => page.locator(`${host} app-speech-player`);

test('only shows Loop when asked to', async ({ page }) => {
    await openPage(page, '/text-to-speech');
    await expect(player(page, 'app-text-to-speech').getByRole('button', { name: 'Speak' })).toBeVisible();
    await expect(player(page, 'app-text-to-speech').getByRole('button', { name: 'Loop' })).toBeHidden();
    await openPage(page, '/phoneme-lab');
    await expect(player(page, 'app-phoneme-lab').getByRole('button', { name: 'Loop' })).toBeVisible();
});

test('starts speaking long text before all of it is rendered', async ({ page }) => {
    await openPage(page, '/text-to-speech');
    const sentences = Array.from({ length: 120 }, (_, i) => `Here is sentence number ${i + 1} of this long text.`);
    await page.locator('app-text-to-speech').getByRole('textbox', { name: 'Text' }).fill(sentences.join(' '));
    const tts = player(page, 'app-text-to-speech');
    await tts.locator('.speak').click();
    await expect(tts.locator('.speak')).toHaveText('Stop');
    // While later parts render, the status counts what is ready so far.
    await expect(tts.getByRole('status')).toHaveText(/so far…$/);
    await expect(page.locator('app-text-to-speech .reading span.now')).toHaveCount(1);
    await tts.locator('.speak').click();
    await expect(tts.locator('.speak')).toHaveText('Speak');
    await expect(page.locator('app-text-to-speech .reading span.now')).toHaveCount(0);
    // Stopping before every part is ready says so, and later parts no longer change the status.
    await expect(tts.getByRole('status')).toHaveText('Stopped.');
    await page.waitForTimeout(500);
    await expect(tts.getByRole('status')).toHaveText('Stopped.');
});

test('splits at sentence ends, not after a title or initials', async ({ page }) => {
    await openPage(page, '/text-to-speech');
    // The first part holds up to 300 characters. Padding puts "Dr. Smith" and "U.S. Army" at its end.
    const padding = 'One two three four five six seven eight nine ten. '.repeat(5);
    const text = `${padding}We met Dr. Smith of the U.S. Army there.${' And more words follow here.'.repeat(12)}`;
    await page.locator('app-text-to-speech').getByRole('textbox', { name: 'Text' }).fill(text);
    await player(page, 'app-text-to-speech').getByRole('button', { name: 'Save WAV' }).click();
    await expect(player(page, 'app-text-to-speech').getByRole('status')).toHaveText(/^Saved/);
    // Read as one sentence: "doctor smith", and "U S Army" with no sentence end after "U S".
    await expect(page.locator('app-text-to-speech .ipa')).toContainText('ˈdɑktɚ ˈsmɪθ əv ðə ˌju ˈɛs ˈɑɹmi ˈðɛɹ.');
});

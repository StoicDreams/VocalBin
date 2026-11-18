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
    // The first part is 300 characters of speech, which takes a while to render on a busy machine. The player sends a
    // reading when it is ready, so wait for that rather than for an amount of time.
    await tts.evaluate(el => { window.firstReading = new Promise(resolve => el.addEventListener('reading', () => resolve(true), { once: true })); });
    await tts.locator('.speak').click();
    await page.evaluate(() => window.firstReading);
    await expect(tts.locator('.speak')).toHaveText('Stop');
    // While later parts render, the status counts what is ready so far.
    await expect(tts.getByRole('status')).toHaveText(/so far…$/);
    await expect(page.locator('app-text-to-speech .reading span.now')).toHaveCount(1);
    await tts.locator('.speak').click();
    await expect(tts.locator('.speak')).toHaveText('Speak');
    await expect(page.locator('app-text-to-speech .reading span.now')).toHaveCount(0);
    // Stopping before every part is ready says so, and later parts no longer change the status.
    await expect(tts.getByRole('status')).toHaveText('Stopped.');
    // The worker renders one part at a time, in the order asked. Ask for one more and wait for it, so every part that
    // was still rendering when Stop was pressed has come back before the status is checked again.
    await tts.evaluate(player => player.renderPart({ ...player.getJob(), text: 'Done.' }, { from: 0, to: 5 }, 44100));
    await expect(tts.getByRole('status')).toHaveText('Stopped.');
});

test('splits at sentence ends, not after a title or initials', async ({ page }) => {
    await openPage(page, '/text-to-speech');
    // The first part holds up to 300 characters. Padding puts "Dr. Smith" and "U.S. Army" at its end.
    const padding = 'One two three four five six seven eight nine ten. '.repeat(5);
    const text = `${padding}We met Dr. Smith of the U.S. Army there.${' And more words follow here.'.repeat(12)}`;
    await page.locator('app-text-to-speech').getByRole('textbox', { name: 'Text' }).fill(text);
    // Rendering this much speech takes a while on a busy machine, so wait for the file itself, which comes when it is done.
    const download = page.waitForEvent('download');
    await player(page, 'app-text-to-speech').getByRole('button', { name: 'Save WAV' }).click();
    await download;
    await expect(player(page, 'app-text-to-speech').getByRole('status')).toHaveText(/^Saved/);
    // Read as one sentence: "doctor smith", and "U S Army" with no sentence end after "U S".
    await expect(page.locator('app-text-to-speech .ipa')).toContainText('ˈdɑktɚ ˈsmɪθ əv ðə ˌju ˈɛs ˈɑɹmi ˈðɛɹ.');
});

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


import { test, expect, openPage } from './support/site.mjs';

// app-voice-controls: the voice presets and sliders shared by the Phoneme Lab and Text to Speech.
const controls = page => page.locator('app-text-to-speech app-voice-controls');
const settings = page => controls(page).evaluate(element => element.settings);

test.beforeEach(async ({ page }) => {
    await openPage(page, '/text-to-speech');
});

test('shows three voices and eight sliders with their values', async ({ page }) => {
    for (const name of ['Man', 'Woman', 'Child']) {
        await expect(controls(page).getByRole('button', { name, exact: true })).toBeVisible();
    }
    await expect(controls(page).getByRole('slider')).toHaveCount(8);
    await expect(controls(page).getByRole('slider', { name: 'Pitch' })).toHaveAttribute('aria-valuetext', '110 Hz');
    await expect(controls(page).locator('output').first()).toHaveText('110 Hz');
});

test('settings give the synthesizer each parameter number and value', async ({ page }) => {
    // Parameter numbers from vocal_synth::param in synth/src/voice.rs.
    expect(await settings(page)).toEqual([[0, 110], [1, 17.5], [2, 1], [3, 0.1], [4, 0.15], [7, 0.8], [10, 1], [11, 1]]);
});

test('presets and sliders change the settings and say so', async ({ page }) => {
    await controls(page).evaluate(element => {
        window.voiceChanges = 0;
        element.addEventListener('input', () => window.voiceChanges++);
    });
    const changes = () => page.evaluate(() => window.voiceChanges);
    await controls(page).getByRole('button', { name: 'Woman' }).click();
    await expect(controls(page).getByRole('slider', { name: 'Pitch' })).toHaveAttribute('aria-valuetext', '210 Hz');
    expect((await settings(page))[0]).toEqual([0, 210]);
    expect(await changes()).toBe(1);
    await controls(page).getByRole('slider', { name: 'Speed' }).fill('1.5');
    await expect(controls(page).getByRole('slider', { name: 'Speed' })).toHaveAttribute('aria-valuetext', '1.50×');
    expect((await settings(page))[6]).toEqual([10, 1.5]);
    // Playwright's fill sends more than one input event for a slider, so only check that it was heard.
    expect(await changes()).toBeGreaterThan(1);
});

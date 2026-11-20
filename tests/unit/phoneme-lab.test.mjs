import { test, expect, openPage } from './support/site.mjs';

// app-phoneme-lab: stage 2 of the speech synthesizer (synth/, built into webapp/root_files/vocal-synth.wasm).
const lab = page => page.locator('app-phoneme-lab');
const text = page => lab(page).getByRole('textbox', { name: 'Phonemes (ARPAbet)' });
const status = page => lab(page).getByRole('status');
const slider = (page, name) => lab(page).getByRole('slider', { name });

test.beforeEach(async ({ page }) => {
    await openPage(page, '/phoneme-lab');
});

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

test('asks for text when there is none', async ({ page }) => {
    await text(page).fill('   ');
    await lab(page).getByRole('button', { name: 'Save WAV' }).click();
    await expect(status(page)).toHaveText('Type some phonemes first, or pick an example.');
});

const ipaText = page => lab(page).getByRole('textbox', { name: 'Phonemes (IPA)' });
const notation = (page, name) => lab(page).getByRole('group', { name: 'Notation' }).getByRole('button', { name });

test('IPA and ARPAbet each keep their own text', async ({ page }) => {
    await notation(page, 'IPA').click();
    await expect(notation(page, 'IPA')).toHaveAttribute('aria-pressed', 'true');
    await expect(notation(page, 'ARPAbet')).toHaveAttribute('aria-pressed', 'false');
    await expect(ipaText(page)).toHaveValue('həˈloʊ, ˈwɝld.');
    await ipaText(page).fill('ˈbi.');
    await notation(page, 'ARPAbet').click();
    await expect(text(page)).toHaveValue('HH AH0 L OW1 , W ER1 L D .');
    await notation(page, 'IPA').click();
    await expect(ipaText(page)).toHaveValue('ˈbi.');
    await lab(page).getByRole('button', { name: 'How are you?' }).click();
    await expect(ipaText(page)).toHaveValue('ˈhaʊ ˈɑɹ ˈju?');
});

test('the IPA chart adds sounds and stress marks with no spaces', async ({ page }) => {
    await notation(page, 'IPA').click();
    await ipaText(page).fill('');
    await lab(page).getByText('Phoneme chart').click();
    await expect(lab(page).getByRole('button', { name: 'AY, aɪ as in hide' })).toHaveCount(0);
    for (const name of ['h as in he', 'ə as in about', 'Primary stress, before the stressed syllable', 'l as in lee', 'oʊ as in oat']) {
        await lab(page).getByRole('button', { name, exact: true }).click();
    }
    await expect(ipaText(page)).toHaveValue('həˈloʊ');
    await notation(page, 'ARPAbet').click();
    await expect(lab(page).getByRole('button', { name: 'ə as in about' })).toBeHidden();
    await expect(lab(page).getByRole('button', { name: 'AY, aɪ as in hide' })).toBeVisible();
});


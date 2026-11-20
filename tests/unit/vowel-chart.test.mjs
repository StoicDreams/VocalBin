import { test, expect, openPage } from './support/site.mjs';

// app-vowel-chart: the vowel space on the How Speech Is Made guide, from the synthesizer's vowel targets.
const chart = page => page.locator('app-vowel-chart');
const point = (page, n) => chart(page).locator('.point').nth(n);

test.beforeEach(async ({ page }) => {
    await openPage(page, '/guides/how-speech-is-made');
});

test('places every vowel, with its numbers in a table', async ({ page }) => {
    await expect(chart(page).locator('.point')).toHaveCount(12);
    await expect(chart(page).getByRole('img', { name: /Vowel chart/ })).toBeVisible();
    await chart(page).getByText('Show the numbers').click();
    const heed = chart(page).getByRole('row', { name: /heed/ });
    await expect(heed).toContainText('342');
    await expect(heed).toContainText('2322');
});

// The chart plots each vowel by its formants, so where a vowel sits is the data it shows, not styling, and tests check it.

test('a shorter vocal tract raises every formant by the same proportion', async ({ page }) => {
    await chart(page).getByRole('button', { name: 'Child' }).click();
    await expect(chart(page).getByRole('button', { name: 'Child' })).toHaveAttribute('aria-pressed', 'true');
    await chart(page).getByText('Show the numbers').click();
    // 342 × 17.5 / 12.5 and 2322 × 17.5 / 12.5.
    const heed = chart(page).getByRole('row', { name: /heed/ });
    await expect(heed).toContainText('479');
    await expect(heed).toContainText('3251');
});

test('hovering a vowel shows its formants', async ({ page }) => {
    await point(page, 0).hover();
    await expect(chart(page).locator('.tip')).toHaveText('/i/ as in “heed”: F1 342 Hz, F2 2322 Hz');
    await expect(chart(page).locator('.tip')).toBeVisible();
});

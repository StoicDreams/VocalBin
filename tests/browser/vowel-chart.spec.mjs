import { test, expect, openPage } from '../support/site.mjs';

// app-vowel-chart: the vowel space on the How Speech Is Made guide, from the synthesizer's vowel targets.
const chart = page => page.locator('app-vowel-chart');
const point = (page, n) => chart(page).locator('.point').nth(n);

test.beforeEach(async ({ page }) => {
    await openPage(page, '/guides/how-speech-is-made');
});

// The chart plots each vowel by its formants, so where a vowel sits is the data it shows, not styling, and tests check it.
test('front vowels sit left and open vowels low, as in the mouth', async ({ page }) => {
    const box = async n => (await point(page, n).boundingBox());
    // 0 is /i/ in "heed", 6 is /ɑ/ in "hod", 10 is /u/ in "who'd".
    const [i, a, u] = [await box(0), await box(6), await box(10)];
    expect(i.x).toBeLessThan(u.x);
    expect(a.y).toBeGreaterThan(i.y);
});


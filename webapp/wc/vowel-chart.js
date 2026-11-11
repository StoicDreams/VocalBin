/*!
 * Vocal Bin Vowel Chart
 * The vowel space: each American English vowel placed by its first two formants, F1 down and F2 from right to left,
 * so the chart matches the mouth (front vowels on the left, open vowels at the bottom). The numbers are the averages
 * for adult men from Hillenbrand, Getty, Clark and Wheeler (1995), the same targets the synthesizer uses
 * (synth/src/vowels.rs). Woman and Child scale them by vocal tract length, as the synthesizer does.
 *
 * <app-vowel-chart></app-vowel-chart>
 *
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // IPA, example word, F1, F2 (Hz), from synth/src/vowels.rs.
    const VOWELS = [
        ['i', 'heed', 342, 2322], ['ɪ', 'hid', 427, 2034], ['e', 'hayed', 476, 2089], ['ɛ', 'head', 580, 1799],
        ['æ', 'had', 588, 1952], ['ʌ', 'hud', 623, 1200], ['ɑ', 'hod', 768, 1333], ['ɔ', 'hawed', 652, 997],
        ['o', 'hoed', 497, 910], ['ʊ', 'hood', 469, 1122], ['u', "who'd", 378, 997], ['ɝ', 'heard', 474, 1379],
    ];
    // Vocal tract lengths in cm, as the voice presets use. Formants scale with 17.5 / length.
    const VOICES = { Man: 17.5, Woman: 14.8, Child: 12.5 };
    const REFERENCE = 17.5;
    // Plot area in SVG units, and the axis ranges, wide enough for a child's voice.
    const W = 560, H = 380, LEFT = 62, RIGHT = 20, TOP = 16, BOTTOM = 52;
    const F2 = [700, 3400], F1 = [250, 1100];
    const x = f2 => LEFT + (F2[1] - f2) / (F2[1] - F2[0]) * (W - LEFT - RIGHT);
    const y = f1 => TOP + (f1 - F1[0]) / (F1[1] - F1[0]) * (H - TOP - BOTTOM);
    const SVG = 'http://www.w3.org/2000/svg';

    function svg(name, attributes = {}, text) {
        const element = document.createElementNS(SVG, name);
        Object.entries(attributes).forEach(([key, value]) => element.setAttribute(key, value));
        if (text !== undefined) element.textContent = text;
        return element;
    }

    const voiceButtons = Object.keys(VOICES).map(name =>
        `<button type="button" data-voice="${name}" aria-pressed="${name === 'Man'}">${name}</button>`).join('');

    webui.define("app-vowel-chart", {
        constructor() {
            const t = this;
            // t.template is emptied once it moves into the shadow root, so find everything needed later now.
            t._plot = t.template.querySelector('svg');
            t._tip = t.template.querySelector('.tip');
            t._rows = t.template.querySelector('tbody');
            t._buttons = [...t.template.querySelectorAll('[data-voice]')];
            t._buttons.forEach(button => button.addEventListener('click', () => t.show(button.dataset.voice)));
            t.drawAxes();
            t._points = VOWELS.map(([symbol, word]) => {
                const group = svg('g', { class: 'point' });
                group.append(svg('circle', { r: 6 }), svg('text', { dx: 10, dy: 5 }, symbol));
                group.addEventListener('pointerenter', () => t.tip(group, symbol, word));
                group.addEventListener('pointerleave', () => { t._tip.hidden = true; });
                t._plot.append(group);
                return group;
            });
            t.show('Man');
        },
        drawAxes() {
            const t = this;
            const axes = svg('g', { class: 'axes', 'aria-hidden': 'true' });
            for (let f2 = 1000; f2 <= 3000; f2 += 500) {
                axes.append(svg('line', { x1: x(f2), x2: x(f2), y1: TOP, y2: H - BOTTOM }));
                axes.append(svg('text', { x: x(f2), y: H - BOTTOM + 20, 'text-anchor': 'middle' }, f2));
            }
            for (let f1 = 300; f1 <= 1100; f1 += 200) {
                axes.append(svg('line', { x1: LEFT, x2: W - RIGHT, y1: y(f1), y2: y(f1) }));
                axes.append(svg('text', { x: LEFT - 8, y: y(f1) + 4, 'text-anchor': 'end' }, f1));
            }
            axes.append(svg('text', { class: 'label', x: (LEFT + W - RIGHT) / 2, y: H - 6, 'text-anchor': 'middle' }, '← F2 (Hz): tongue further forward'));
            axes.append(svg('text', { class: 'label', x: 16, y: (TOP + H - BOTTOM) / 2, 'text-anchor': 'middle', transform: `rotate(-90 16 ${(TOP + H - BOTTOM) / 2})` }, 'F1 (Hz): mouth more open →'));
            t._plot.append(axes);
        },
        /** Moves the vowels to a voice's formants, and fills the table. */
        show(voice) {
            const t = this;
            const scale = REFERENCE / VOICES[voice];
            t._voice = voice;
            t._buttons.forEach(button => button.setAttribute('aria-pressed', String(button.dataset.voice === voice)));
            const rows = VOWELS.map(([symbol, word, f1, f2], i) => {
                const [a, b] = [Math.round(f1 * scale), Math.round(f2 * scale)];
                // A CSS transform, which can move smoothly, where an SVG transform attribute would jump.
                t._points[i].style.transform = `translate(${x(b).toFixed(1)}px, ${y(a).toFixed(1)}px)`;
                t._points[i].dataset.f1 = a;
                t._points[i].dataset.f2 = b;
                const row = document.createElement('tr');
                [symbol, word, a, b].forEach((value, n) => {
                    const cell = document.createElement(n === 0 ? 'th' : 'td');
                    if (n === 0) cell.setAttribute('scope', 'row');
                    cell.textContent = value;
                    row.append(cell);
                });
                return row;
            });
            t._rows.replaceChildren(...rows);
            t._tip.hidden = true;
        },
        tip(point, symbol, word) {
            const t = this;
            t._tip.textContent = `/${symbol}/ as in “${word}”: F1 ${point.dataset.f1} Hz, F2 ${point.dataset.f2} Hz`;
            const box = point.getBoundingClientRect();
            const host = t.getBoundingClientRect();
            t._tip.style.left = `${box.left - host.left + box.width / 2}px`;
            t._tip.style.top = `${box.top - host.top}px`;
            t._tip.hidden = false;
        },
        shadowTemplate: `
<div class="voices" role="group" aria-label="Voice">${voiceButtons}</div>
<svg viewBox="0 0 ${W} ${H}" role="img" aria-label="Vowel chart: each vowel placed by its first two formants. The numbers are in the table below."></svg>
<div class="tip" role="presentation" hidden></div>
<details>
<summary>Show the numbers</summary>
<table>
<thead><tr><th scope="col">Vowel</th><th scope="col">As in</th><th scope="col">F1 (Hz)</th><th scope="col">F2 (Hz)</th></tr></thead>
<tbody></tbody>
</table>
</details>
<style type="text/css">
:host {
display:grid;
gap:0.5em;
position:relative;
}
.voices {
display:flex;
flex-wrap:wrap;
gap:0.5em;
}
button {
font:inherit;
cursor:pointer;
border:1px solid var(--color-primary);
border-radius:0.4em;
background-color:var(--color-background-offset, #fff);
color:var(--color-primary);
padding:0.3em 0.9em;
}
button[aria-pressed="true"] {
background-color:var(--color-primary);
color:var(--color-primary-offset, #fff);
}
button:focus-visible,
summary:focus-visible {
outline:2px solid var(--color-focus, currentColor);
outline-offset:2px;
}
svg {
width:100%;
height:auto;
max-width:44em;
overflow:visible;
}
.axes line {
stroke:currentColor;
stroke-opacity:0.15;
stroke-width:1;
}
.axes text {
fill:currentColor;
fill-opacity:0.75;
font-size:16px;
}
.axes .label {
fill-opacity:0.9;
font-size:16px;
}
.point {
transition:transform 0.4s ease;
}
.point circle {
fill:var(--color-title, #d10694);
stroke:var(--color-background, #333);
stroke-width:2;
}
.point text {
fill:currentColor;
font-size:24px;
}
@media (prefers-reduced-motion: reduce) {
.point {
transition:none;
}
}
.tip {
position:absolute;
transform:translate(-50%, calc(-100% - 8px));
pointer-events:none;
white-space:nowrap;
padding:0.3em 0.6em;
border-radius:0.3em;
font-size:0.9em;
background-color:var(--color-black, #101010);
color:var(--color-white, #fff);
}
.tip[hidden] {
display:none;
}
summary {
cursor:pointer;
}
table {
border-collapse:collapse;
margin-top:0.5em;
}
th, td {
padding:0.2em 0.8em;
text-align:right;
border-bottom:1px solid color-mix(in srgb, currentColor 20%, transparent);
}
th[scope="row"], thead th:first-child, td:nth-child(2) {
text-align:left;
}
</style>
`
    });
}

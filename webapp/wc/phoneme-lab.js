/*!
 * Vocal Bin Phoneme Lab
 * Stage 2 of the Vocal Bin speech synthesizer: type phonemes in ARPAbet or IPA, hear them spoken, and save a WAV file.
 * The sound comes from synth/ (Rust, compiled to vocal-synth.wasm), through app-speech-player.
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // The same phonemes as ARPABET in synth/src/english.rs: symbol, IPA, example word.
    const VOWELS = [
        ['AA', 'ɑ', 'odd'], ['AE', 'æ', 'at'], ['AH', 'ʌ', 'hut'], ['AO', 'ɔ', 'ought'], ['AW', 'aʊ', 'cow'],
        ['AY', 'aɪ', 'hide'], ['EH', 'ɛ', 'Ed'], ['ER', 'ɝ', 'hurt'], ['EY', 'eɪ', 'ate'], ['IH', 'ɪ', 'it'],
        ['IY', 'i', 'eat'], ['OW', 'oʊ', 'oat'], ['OY', 'ɔɪ', 'toy'], ['UH', 'ʊ', 'hood'], ['UW', 'u', 'two'],
    ];
    const CONSONANTS = [
        ['B', 'b', 'be'], ['CH', 'tʃ', 'cheese'], ['D', 'd', 'dee'], ['DH', 'ð', 'thee'], ['F', 'f', 'fee'],
        ['G', 'ɡ', 'green'], ['HH', 'h', 'he'], ['JH', 'dʒ', 'gee'], ['K', 'k', 'key'], ['L', 'l', 'lee'],
        ['M', 'm', 'me'], ['N', 'n', 'knee'], ['NG', 'ŋ', 'ping'], ['P', 'p', 'pee'], ['R', 'ɹ', 'read'],
        ['S', 's', 'sea'], ['SH', 'ʃ', 'she'], ['T', 't', 'tea'], ['TH', 'θ', 'theta'], ['V', 'v', 'vee'],
        ['W', 'w', 'we'], ['Y', 'j', 'yield'], ['Z', 'z', 'zee'], ['ZH', 'ʒ', 'seizure'],
    ];
    // Sounds only IPA writes as their own symbols (ARPAbet writes them AH0 and ER0), and IPA's marks.
    const IPA_ONLY = [['ə', 'ə as in about'], ['ɚ', 'ɚ as in butter'], ['ˈ', 'Primary stress, before the stressed syllable'], ['ˌ', 'Secondary stress']];
    // Each example in ARPAbet and in IPA. Both must sound the same.
    const EXAMPLES = [
        ['Hello, world.', 'HH AH0 L OW1 , W ER1 L D .', 'həˈloʊ, ˈwɝld.'],
        ['How are you?', 'HH AW1 AA1 R Y UW1 ?', 'ˈhaʊ ˈɑɹ ˈju?'],
        ['Speech synthesis.', 'S P IY1 CH , S IH1 N TH AH0 S AH0 S .', 'ˈspitʃ, ˈsɪnθəsəs.'],
        ['Vocal Bin.', 'V OW1 K AH0 L , B IH1 N .', 'ˈvoʊkəl, ˈbɪn.'],
        ['She sells sea shells.', 'SH IY1 S EH1 L Z S IY1 SH EH1 L Z .', 'ʃi sɛlz si ʃɛlz.'],
    ];
    const NOTATIONS = {
        arpabet: {
            label: 'Phonemes (ARPAbet)',
            capitalize: 'characters',
            hint: 'Separate phonemes with spaces. Vowels take a stress number: 1 stressed, 2 secondary, 0 unstressed. A comma pauses, and a period or question mark ends a sentence. Ctrl+Enter speaks. Loop repeats the speech and picks up your changes on each pass.',
            chartHint: 'Each button adds its phoneme at the cursor. Vowels are added stressed.',
            error: bad => `“${bad}” isn't an ARPAbet phoneme. Vowels take a stress number (0, 1, or 2), and consonants take none.`,
        },
        ipa: {
            label: 'Phonemes (IPA)',
            capitalize: 'off',
            hint: 'Write each word without spaces between its sounds. Put ˈ (or \') before the stressed syllable and ˌ before a secondary one. A word with no marks stresses its first full vowel. Slashes, brackets, and length marks are fine. A comma pauses, and a period or question mark ends a sentence. Ctrl+Enter speaks. Loop repeats the speech and picks up your changes on each pass.',
            chartHint: 'Each button adds its sound or mark at the cursor, with no spaces. Add a space between words.',
            error: bad => `“${bad}” isn't a sound this English voice knows. The chart shows the IPA it reads.`,
        },
    };
    const chartButtons = (list, vowel) => list.map(([symbol, ipa, word]) =>
        `<button type="button" data-symbol="${symbol}${vowel ? '1' : ''}" data-ipa="${ipa}" data-word="${word}" aria-label="${symbol}, ${ipa} as in ${word}" title="${ipa} as in ${word}"><span class="sym">${symbol}</span><span class="ipa">${ipa}</span></button>`
    ).join('');
    const ipaOnlyButtons = IPA_ONLY.map(([ipa, name]) =>
        `<button type="button" class="ipa-only" data-ipa="${ipa}" aria-label="${name}" title="${name}" hidden><span class="ipa">${ipa}</span></button>`
    ).join('');
    const exampleButtons = EXAMPLES.map(([label], i) => `<button type="button" data-example="${i}">${label}</button>`).join('');

    webui.define("app-phoneme-lab", {
        constructor() {
            const t = this;
            t._notation = 'arpabet';
            t._texts = { arpabet: EXAMPLES[0][1], ipa: EXAMPLES[0][2] };
            // t.template is emptied once it moves into the shadow root, so find everything needed later now.
            t._text = t.template.querySelector('textarea');
            t._textLabel = t.template.querySelector('.text span');
            t._hint = t.template.querySelector('#hint');
            t._chartHint = t.template.querySelector('.chart .hint');
            t._notationButtons = [...t.template.querySelectorAll('[data-notation]')];
            t._chartSymbols = [...t.template.querySelectorAll('.chart [data-symbol]')];
            t._ipaOnly = [...t.template.querySelectorAll('.ipa-only')];
            t._timeline = t.template.querySelector('.timeline');
            t._player = t.template.querySelector('app-speech-player');
            t._voice = t.template.querySelector('app-voice-controls');
            t._text.value = t._texts.arpabet;
            t._notationButtons.forEach(button => {
                button.addEventListener('click', () => t.setNotation(button.dataset.notation));
            });
            t._text.addEventListener('input', () => {
                t._text.removeAttribute('aria-invalid');
                t.changed();
            });
            t._voice.addEventListener('input', () => t.changed());
            t._text.addEventListener('keydown', ev => {
                if (ev.key === 'Enter' && (ev.ctrlKey || ev.metaKey)) {
                    ev.preventDefault();
                    t._player.toggle?.();
                }
            });
            t.template.querySelectorAll('.chart button').forEach(button => {
                button.addEventListener('click', () => {
                    if (t._notation === 'ipa') t.insertIpa(button.dataset.ipa);
                    else t.insert(button.dataset.symbol);
                });
            });
            t.template.querySelectorAll('[data-example]').forEach(button => {
                button.addEventListener('click', () => {
                    t._text.value = EXAMPLES[Number(button.dataset.example)][t._notation === 'ipa' ? 2 : 1];
                    t._text.removeAttribute('aria-invalid');
                    t.changed();
                });
            });
            t._player.getJob = () => ({ text: t._text.value, notation: t._notation, settings: t._voice.settings });
            t._player.describe = reading => {
                const count = reading.items.filter(item => !/^[,.!?|‖]$/.test(reading.text.slice(item.from, item.to))).length;
                return `${count} phonemes, ${reading.seconds.toFixed(1)} seconds${reading.done ? '.' : ' so far…'}`;
            };
            t._player.parseErrorMessage = (job, error) => t.showParseError(job, error);
            t._player.addEventListener('reading', ev => t.showTimeline(ev.detail));
            t._player.addEventListener('item', ev => t.highlight(ev.detail));
        },
        /** Tells the player the text or settings changed, so a loop renders its next pass ahead of time. */
        changed() {
            this._player.changed?.();
        },
        /**
         * Switches between ARPAbet and IPA. Each notation keeps its own text, so switching back finds it as it was.
         */
        setNotation(notation) {
            const t = this;
            if (!(notation in NOTATIONS) || notation === t._notation) return;
            t._texts[t._notation] = t._text.value;
            t._notation = notation;
            const ipa = notation === 'ipa';
            const about = NOTATIONS[notation];
            t._notationButtons.forEach(button => {
                button.setAttribute('aria-pressed', String(button.dataset.notation === notation));
            });
            t._text.value = t._texts[notation];
            t._text.removeAttribute('aria-invalid');
            t._text.setAttribute('autocapitalize', about.capitalize);
            t._textLabel.textContent = about.label;
            t._hint.textContent = about.hint;
            t._chartHint.textContent = about.chartHint;
            t._chartSymbols.forEach(button => {
                const { symbol, ipa: sound, word } = button.dataset;
                button.setAttribute('aria-label', ipa ? `${sound} as in ${word}` : `${symbol.replace(/1$/, '')}, ${sound} as in ${word}`);
                button.querySelector('.sym').hidden = ipa;
            });
            t._ipaOnly.forEach(element => { element.hidden = !ipa; });
            t.changed();
        },
        /** Inserts IPA at the cursor as it is. Sounds in a word need no spaces. */
        insertIpa(symbol) {
            const t = this;
            const text = t._text.value;
            const start = t._text.selectionStart ?? text.length;
            const end = t._text.selectionEnd ?? text.length;
            t._text.value = `${text.slice(0, start)}${symbol}${text.slice(end)}`;
            const cursor = start + symbol.length;
            t._text.setSelectionRange(cursor, cursor);
            t._text.removeAttribute('aria-invalid');
            t.changed();
        },
        /** Inserts a phoneme at the cursor, with spaces around it. */
        insert(symbol) {
            const t = this;
            const text = t._text.value;
            const start = t._text.selectionStart ?? text.length;
            const end = t._text.selectionEnd ?? text.length;
            const before = text.slice(0, start);
            const after = text.slice(end);
            const lead = before && !/\s$/.test(before) ? ' ' : '';
            const trail = after && /^\s/.test(after) ? '' : ' ';
            t._text.value = `${before}${lead}${symbol}${trail}${after}`;
            const cursor = before.length + lead.length + symbol.length + trail.length;
            t._text.setSelectionRange(cursor, cursor);
            t._text.removeAttribute('aria-invalid');
            t.changed();
        },
        /** Selects the text that can't be read, and returns why for the status. */
        showParseError(job, { start, end }) {
            const t = this;
            // The text box may have changed to the other notation while this rendered.
            if (job.notation === t._notation && t._text.value === job.text) {
                t._text.setAttribute('aria-invalid', 'true');
                t._text.setSelectionRange(start, end);
            }
            return NOTATIONS[job.notation].error(job.text.slice(start, end));
        },
        showTimeline(reading) {
            const t = this;
            const chips = reading.items.map(item => {
                const chip = document.createElement('span');
                chip.textContent = reading.text.slice(item.from, item.to);
                chip.style.flexGrow = String(item.end - item.start);
                return chip;
            });
            t._current = null;
            t._timeline.replaceChildren(...chips);
            t._chips = chips;
        },
        /** Highlights the phoneme being spoken, or none for -1. */
        highlight(index) {
            const t = this;
            const chip = index >= 0 ? t._chips?.[index] || null : null;
            if (chip === t._current) return;
            t._current?.classList.remove('now');
            chip?.classList.add('now');
            t._current = chip;
        },
        shadowTemplate: `
<div class="row notation" role="group" aria-label="Notation"><span>Write in</span><button type="button" data-notation="arpabet" aria-pressed="true">ARPAbet</button><button type="button" data-notation="ipa" aria-pressed="false">IPA</button></div>
<label class="text"><span>Phonemes (ARPAbet)</span><textarea rows="3" maxlength="4000" spellcheck="false" autocapitalize="characters" aria-describedby="hint"></textarea></label>
<p id="hint" class="hint">Separate phonemes with spaces. Vowels take a stress number: 1 stressed, 2 secondary, 0 unstressed. A comma pauses, and a period or question mark ends a sentence. Ctrl+Enter speaks. Loop repeats the speech and picks up your changes on each pass.</p>
<div class="row examples" role="group" aria-label="Examples">${exampleButtons}</div>
<app-speech-player loop file-prefix="vocal-bin-phoneme-lab" busy-text="Rendering…" empty-text="Type some phonemes first, or pick an example."></app-speech-player>
<div class="timeline" aria-hidden="true"></div>
<details class="chart">
<summary>Phoneme chart</summary>
<p class="hint">Each button adds its phoneme at the cursor. Vowels are added stressed.</p>
<fieldset><legend>Vowels</legend>${chartButtons(VOWELS, true)}</fieldset>
<fieldset><legend>Consonants</legend>${chartButtons(CONSONANTS, false)}</fieldset>
<fieldset class="ipa-only" hidden><legend>Reduced vowels and stress</legend>${ipaOnlyButtons}</fieldset>
</details>
<app-voice-controls></app-voice-controls>
<style type="text/css">
:host {
display:grid;
gap:var(--padding, 1em);
}
.text {
display:grid;
gap:0.3em;
}
textarea {
font:inherit;
font-family:ui-monospace, "Cascadia Code", Menlo, Consolas, monospace;
font-size:1.1em;
padding:0.5em;
border-radius:0.4em;
border:1px solid var(--color-primary);
background-color:var(--color-background-offset, #fff);
color:var(--color-black, #101010);
resize:vertical;
box-sizing:border-box;
width:100%;
}
textarea[aria-invalid="true"] {
border:2px solid var(--color-danger, #b70505);
}
.hint {
margin:0;
font-size:0.9em;
opacity:0.85;
}
.row {
display:flex;
flex-wrap:wrap;
align-items:center;
gap:0.5em;
}
button {
font:inherit;
cursor:pointer;
border:1px solid var(--color-primary);
border-radius:0.4em;
background-color:var(--color-background-offset, #fff);
color:var(--color-primary);
padding:0.5em 1em;
}
button:focus-visible,
textarea:focus-visible,
summary:focus-visible {
outline:2px solid var(--color-focus, currentColor);
outline-offset:2px;
}
[hidden] {
display:none !important;
}
.notation button {
padding:0.3em 0.9em;
}
.notation button[aria-pressed="true"] {
background-color:var(--color-primary);
color:var(--color-primary-offset, #fff);
}
.examples button {
padding:0.3em 0.7em;
font-size:0.9em;
}
.timeline {
display:flex;
gap:2px;
min-height:2.2em;
overflow:hidden;
border-radius:0.4em;
background-color:var(--color-black, #101010);
padding:3px;
}
.timeline:empty {
display:none;
}
.timeline span {
flex-basis:0;
min-width:0;
display:flex;
align-items:center;
justify-content:center;
font-size:0.75em;
overflow:hidden;
white-space:nowrap;
border-radius:0.25em;
background-color:color-mix(in srgb, var(--color-title, #d10694) 25%, transparent);
color:var(--color-white, #fff);
}
.timeline span.now {
background-color:var(--color-title, #d10694);
}
.chart summary {
cursor:pointer;
padding:0.3em 0;
}
fieldset {
border:1px solid color-mix(in srgb, currentColor 30%, transparent);
border-radius:0.4em;
margin:0;
padding:var(--padding, 1em);
display:flex;
flex-wrap:wrap;
gap:0.5em;
}
legend {
padding:0 0.4em;
}
.chart fieldset {
margin-top:0.5em;
}
.chart button {
display:grid;
justify-items:center;
min-width:3.6em;
padding:0.25em 0.4em;
}
.sym {
font-weight:bold;
}
.ipa {
font-size:0.85em;
}
</style>
`
    });
}

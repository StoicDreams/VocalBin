/*!
 * Vocal Bin Text to Speech
 * Stage 3 of the Vocal Bin speech synthesizer: type English, hear it read with intonation, follow along word by word,
 * and save a WAV file. The sound comes from synth/ (Rust, compiled to vocal-synth.wasm) through app-speech-player,
 * with the pronouncing dictionary vocal-synth-en-us.dict (from the CMU Pronouncing Dictionary).
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    const EXAMPLES = [
        ['Hello!', 'Hello! Welcome to Vocal Bin. How are you today?'],
        ['Numbers', 'On Nov. 11, 2026 at 3:30 PM, Dr. Smith paid $5.50 for 2 coffees, about 25% more than in 1999.'],
        ['Questions', 'Is it raining? Where did you leave the umbrella?'],
        ['Same spelling', 'I will record a new record. They live near the live music venue. Did you read what I read yesterday?'],
        ['About us', 'Vocal Bin is a speech synthesizer we wrote from scratch, with no recordings and no AI.'],
    ];
    const exampleButtons = EXAMPLES.map(([label], i) => `<button type="button" data-example="${i}">${label}</button>`).join('');
    /** Whether an item of the reading is a word, not a pause. */
    const isWord = (text, item) => item.to > item.from && /\w/.test(text.slice(item.from, item.to));

    webui.define("app-text-to-speech", {
        constructor() {
            const t = this;
            // t.template is emptied once it moves into the shadow root, so find everything needed later now.
            t._text = t.template.querySelector('textarea');
            t._player = t.template.querySelector('app-speech-player');
            t._voice = t.template.querySelector('app-voice-controls');
            t._reading = t.template.querySelector('.reading');
            t._ipa = t.template.querySelector('.ipa');
            t._text.value = EXAMPLES[0][1];
            t._text.addEventListener('input', () => t._text.removeAttribute('aria-invalid'));
            t._text.addEventListener('keydown', ev => {
                if (ev.key === 'Enter' && (ev.ctrlKey || ev.metaKey)) {
                    ev.preventDefault();
                    t._player.toggle?.();
                }
            });
            t.template.querySelectorAll('[data-example]').forEach(button => {
                button.addEventListener('click', () => {
                    t._text.value = EXAMPLES[Number(button.dataset.example)][1];
                    t._text.removeAttribute('aria-invalid');
                });
            });
            t._player.getJob = () => ({ text: t._text.value, notation: 'text', settings: t._voice.settings });
            t._player.describe = reading => {
                const words = new Set(reading.items.filter(item => isWord(reading.text, item)).map(item => item.from)).size;
                const amount = `${words} ${words === 1 ? 'word' : 'words'}, ${reading.seconds.toFixed(1)} seconds`;
                return reading.done ? `${amount}.` : `${amount} so far…`;
            };
            t._player.parseErrorMessage = (job, error) => t.showTooLong(job.text, error);
            t._player.addEventListener('reading', ev => t.showReading(ev.detail));
            t._player.addEventListener('item', ev => t.highlight(ev.detail));
        },
        /** The only text error is length, when one sentence is too long: selects it and explains. */
        showTooLong(text, { start, end }) {
            const t = this;
            if (t._text.value === text) {
                t._text.setAttribute('aria-invalid', 'true');
                t._text.setSelectionRange(start, end);
                t._text.focus();
            }
            return 'That sentence is more than we can say in one go. The selected text is past the limit, so try breaking it up.';
        },
        /**
         * Shows the text as it will be read, with a span for each word to highlight while it is spoken, and the IPA
         * transcript. Long text arrives in parts, and the text not yet read is shown plain until it is.
         */
        showReading(reading) {
            const t = this;
            const { text, items } = reading;
            const spans = new Map();
            const parts = [];
            let at = 0;
            const ranges = [...new Map(items.filter(item => isWord(text, item)).map(item => [item.from, item])).values()]
                .sort((a, b) => a.from - b.from);
            ranges.forEach(range => {
                if (range.from < at) return;
                if (range.from > at) parts.push(document.createTextNode(text.slice(at, range.from)));
                const span = document.createElement('span');
                span.textContent = text.slice(range.from, range.to);
                parts.push(span);
                spans.set(range.from, span);
                at = range.to;
            });
            if (at < text.length) parts.push(document.createTextNode(text.slice(at)));
            t._current = null;
            t._reading.replaceChildren(...parts);
            t._words = items.map(item => spans.get(item.from) || null);
            t._ipa.textContent = reading.transcript;
        },
        /** Highlights the word being spoken. Pauses keep the last word lit, and -1 clears it. */
        highlight(index) {
            const t = this;
            const word = index >= 0 ? t._words?.[index] : null;
            if (index >= 0 && !word) return;
            if (word === t._current) return;
            t._current?.classList.remove('now');
            word?.classList.add('now');
            t._current = word;
        },
        shadowTemplate: `
<label class="text"><span>Text</span><textarea rows="6" maxlength="10000" aria-describedby="hint"></textarea></label>
<p id="hint" class="hint">Type or paste English. Numbers, dates, money, and common abbreviations are read as words. Ctrl+Enter speaks.</p>
<div class="row examples" role="group" aria-label="Examples">${exampleButtons}</div>
<app-speech-player dictionary file-prefix="vocal-bin-speech" busy-text="Reading…" empty-text="Type some text first, or pick an example."></app-speech-player>
<p class="reading" aria-hidden="true"></p>
<details class="pronunciation">
<summary>Pronunciation</summary>
<p class="hint">What the voice says, in IPA. You can copy it into the <a href="/phoneme-lab">Phoneme Lab</a> to change how a word sounds.</p>
<p class="ipa" lang="en-fonipa"></p>
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
.hint a {
color:inherit;
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
summary:focus-visible,
a:focus-visible {
outline:2px solid var(--color-focus, currentColor);
outline-offset:2px;
}
.examples button {
padding:0.3em 0.7em;
font-size:0.9em;
}
.reading {
margin:0;
padding:0.6em 0.8em;
border-radius:0.4em;
background-color:var(--color-black, #101010);
color:var(--color-white, #fff);
line-height:1.7;
white-space:pre-wrap;
max-height:16em;
overflow:auto;
}
.reading:empty {
display:none;
}
.reading span {
border-radius:0.25em;
padding:0.1em 0;
}
.reading span.now {
background-color:var(--color-title, #d10694);
}
.pronunciation summary {
cursor:pointer;
padding:0.3em 0;
}
.ipa {
margin:0.5em 0 0;
font-size:1.1em;
line-height:1.6;
user-select:all;
}
.ipa:empty::before {
content:"Speak or save some text to see it here.";
opacity:0.7;
font-size:0.9rem;
}
</style>
`
    });
}

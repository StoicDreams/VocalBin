/*!
 * Vocal Bin Text to Speech
 * Stage 3 of the Vocal Bin speech synthesizer: type English, hear it read with intonation, follow along word by word,
 * and save a WAV file. The sound comes from synth/ (Rust, compiled to vocal-synth.wasm), rendered in a Web Worker,
 * with the pronouncing dictionary vocal-synth-en-us.dict (from the CMU Pronouncing Dictionary).
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // Parameter numbers, the same as vocal_synth::param in synth/src/voice.rs.
    const PARAM = { pitch: 0, tract: 1, rd: 2, breath: 3, roughness: 4, volume: 7, rate: 10, intonation: 11 };
    const EXAMPLES = [
        ['Hello!', 'Hello! Welcome to Vocal Bin. How are you today?'],
        ['Numbers', 'On Nov. 11, 2026 at 3:30 PM, Dr. Smith paid $5.50 for 2 coffees, about 25% more than in 1999.'],
        ['Questions', 'Is it raining? Where did you leave the umbrella?'],
        ['Same spelling', 'I will record a new record. They live near the live music venue. Did you read what I read yesterday?'],
        ['About us', 'Vocal Bin is a speech synthesizer we wrote from scratch, with no recordings and no AI.'],
    ];
    const percent = v => `${Math.round(v * 100)}%`;
    const quality = v => `${v < 0.8 ? 'pressed' : v > 1.6 ? 'lax' : 'modal'} (${v.toFixed(2)})`;
    const SLIDERS = [
        { key: 'pitch', label: 'Pitch', min: 50, max: 400, step: 1, value: 110, show: v => `${v} Hz` },
        { key: 'rate', label: 'Speed', min: 0.5, max: 2, step: 0.05, value: 1, show: v => `${v.toFixed(2)}×` },
        { key: 'intonation', label: 'Intonation', min: 0, max: 2, step: 0.05, value: 1, show: percent },
        { key: 'tract', label: 'Vocal tract length', min: 10, max: 22, step: 0.1, value: 17.5, show: v => `${v.toFixed(1)} cm` },
        { key: 'rd', label: 'Voice quality', min: 0.3, max: 2.7, step: 0.05, value: 1, show: quality },
        { key: 'breath', label: 'Breathiness', min: 0, max: 1, step: 0.01, value: 0.1, show: percent },
        { key: 'roughness', label: 'Roughness', min: 0, max: 1, step: 0.01, value: 0.15, show: percent },
        { key: 'volume', label: 'Volume', min: 0, max: 1, step: 0.01, value: 0.8, show: percent },
    ];
    const PRESETS = {
        Man: { pitch: 110, tract: 17.5, rd: 1, breath: 0.1 },
        Woman: { pitch: 210, tract: 14.8, rd: 1.4, breath: 0.2 },
        Child: { pitch: 290, tract: 12.5, rd: 1.3, breath: 0.15 },
    };
    const DEFAULT_SAMPLE_RATE = 44100;

    /** Fetches a site file once per visit. */
    function once(url, read) {
        let promise = null;
        return () => {
            if (!promise) {
                promise = fetch(url).then(response => {
                    if (!response.ok) throw new Error(`${url}: ${response.status}`);
                    return read(response);
                });
                promise.catch(() => { promise = null; });
            }
            return promise;
        };
    }
    const loadWasm = once('/vocal-synth.wasm', response => response.arrayBuffer());
    const loadDictionary = once('/vocal-synth-en-us.dict', response => response.arrayBuffer());
    // The worker script loads from a Blob URL, so its request takes the same path as other site files.
    const loadWorkerUrl = once('/vocal-synth-speaker-worker.min.js', response => response.text()
        .then(code => URL.createObjectURL(new Blob([code], { type: 'text/javascript' }))));

    // One worker renders for every text-to-speech box on the page. Its first job carries the wasm and dictionary.
    let worker = null;
    let nextJob = 1;
    const jobs = new Map();
    async function renderInWorker(job) {
        if (!worker) {
            const [bytes, dictionary, url] = await Promise.all([loadWasm(), loadDictionary(), loadWorkerUrl()]);
            if (!worker) {
                worker = new Worker(url);
                worker.onmessage = ev => {
                    const done = jobs.get(ev.data.id);
                    jobs.delete(ev.data.id);
                    if (!done) return;
                    if (ev.data.error) done.reject(new Error(ev.data.error));
                    else done.resolve(ev.data);
                };
                worker.onerror = ev => {
                    jobs.forEach(done => done.reject(new Error(ev.message || 'Speaker worker failed')));
                    jobs.clear();
                    worker = null;
                };
                worker.postMessage({ id: 0, wasm: bytes, dictionary, settings: [], text: '', notation: 'text', sampleRate: DEFAULT_SAMPLE_RATE });
            }
        }
        const id = nextJob++;
        return new Promise((resolve, reject) => {
            jobs.set(id, { resolve, reject });
            worker.postMessage({ id, ...job });
        });
    }

    /** 16-bit mono WAV from samples between -1 and 1. */
    function encodeWav(samples, sampleRate) {
        const view = new DataView(new ArrayBuffer(44 + samples.length * 2));
        const text = (offset, value) => {
            for (let i = 0; i < value.length; i++) view.setUint8(offset + i, value.charCodeAt(i));
        };
        text(0, 'RIFF');
        view.setUint32(4, 36 + samples.length * 2, true);
        text(8, 'WAVE');
        text(12, 'fmt ');
        view.setUint32(16, 16, true);
        view.setUint16(20, 1, true);
        view.setUint16(22, 1, true);
        view.setUint32(24, sampleRate, true);
        view.setUint32(28, sampleRate * 2, true);
        view.setUint16(32, 2, true);
        view.setUint16(34, 16, true);
        text(36, 'data');
        view.setUint32(40, samples.length * 2, true);
        for (let i = 0; i < samples.length; i++) {
            const s = Math.max(-1, Math.min(1, samples[i]));
            view.setInt16(44 + i * 2, s < 0 ? s * 0x8000 : s * 0x7fff, true);
        }
        return new Blob([view.buffer], { type: 'audio/wav' });
    }

    const exampleButtons = EXAMPLES.map(([label], i) => `<button type="button" data-example="${i}">${label}</button>`).join('');
    const presetButtons = Object.keys(PRESETS).map(name => `<button type="button" data-preset="${name}">${name}</button>`).join('');
    const sliders = SLIDERS.map(s =>
        `<label class="slider"><span class="name">${s.label}</span><output aria-hidden="true"></output><input type="range" data-key="${s.key}" aria-label="${s.label}" min="${s.min}" max="${s.max}" step="${s.step}" value="${s.value}"></label>`
    ).join('');

    webui.define("app-text-to-speech", {
        constructor() {
            const t = this;
            t._values = {};
            // t.template is emptied once it moves into the shadow root, so find everything needed later now.
            t._text = t.template.querySelector('textarea');
            t._speak = t.template.querySelector('.speak');
            t._save = t.template.querySelector('.save');
            t._status = t.template.querySelector('.status');
            t._reading = t.template.querySelector('.reading');
            t._ipa = t.template.querySelector('.ipa');
            t._inputs = [...t.template.querySelectorAll('input[type=range]')];
            t._text.value = EXAMPLES[0][1];
            t._inputs.forEach(input => {
                t.showValue(input);
                input.addEventListener('input', () => t.showValue(input));
            });
            t._text.addEventListener('input', () => t._text.removeAttribute('aria-invalid'));
            t._text.addEventListener('keydown', ev => {
                if (ev.key === 'Enter' && (ev.ctrlKey || ev.metaKey)) {
                    ev.preventDefault();
                    t.toggleSpeak();
                }
            });
            t.template.querySelectorAll('[data-example]').forEach(button => {
                button.addEventListener('click', () => {
                    t._text.value = EXAMPLES[Number(button.dataset.example)][1];
                    t._text.removeAttribute('aria-invalid');
                });
            });
            t.template.querySelectorAll('[data-preset]').forEach(button => {
                button.addEventListener('click', () => t.applyPreset(button.dataset.preset));
            });
            t._speak.addEventListener('click', () => t.toggleSpeak());
            t._save.addEventListener('click', () => t.saveWav());
        },
        connected() {
            // Start fetching the dictionary now, so the first Speak doesn't wait for it.
            loadDictionary().catch(() => { });
        },
        disconnected() {
            const t = this;
            // Moving the element fires disconnected then reconnected. Only shut down if it has really left the page.
            setTimeout(() => {
                if (!t.isConnected) t.shutdown();
            }, 0);
        },
        showValue(input) {
            const slider = SLIDERS.find(s => s.key === input.dataset.key);
            const value = Number(input.value);
            const text = slider.show(value);
            input.previousElementSibling.textContent = text;
            input.setAttribute('aria-valuetext', text);
            this._values[slider.key] = value;
        },
        applyPreset(name) {
            const t = this;
            const preset = PRESETS[name];
            t._inputs.forEach(input => {
                if (input.dataset.key in preset) {
                    input.value = preset[input.dataset.key];
                    t.showValue(input);
                }
            });
        },
        /** Renders the text with the current settings, or reuses the last render if nothing changed. */
        async render(sampleRate) {
            const t = this;
            const text = t._text.value;
            if (!text.trim()) {
                t._status.textContent = 'Type some text first, or pick an example.';
                return null;
            }
            const settings = Object.keys(PARAM).map(key => [PARAM[key], t._values[key]]);
            const key = JSON.stringify([text, settings, sampleRate]);
            if (t._rendered?.key !== key) {
                if (t._pending?.key !== key) {
                    t._pending = { key, job: renderInWorker({ text, notation: 'text', settings, sampleRate }) };
                }
                const pending = t._pending;
                t._status.textContent = 'Reading…';
                let result;
                try {
                    result = await pending.job;
                } catch (error) {
                    console.error('Text to speech rendering failed', error);
                    t._status.textContent = 'Sorry, the speech could not be rendered in this browser.';
                    return null;
                } finally {
                    if (t._pending === pending) t._pending = null;
                }
                if (result.parseError) return t.showTooLong(text, result.parseError);
                // A newer render may have finished while this one waited.
                if (t._rendered?.key !== key) t._rendered = { key, text, ...result };
            }
            const rendered = t._rendered;
            if (t._shown !== rendered) {
                t._shown = rendered;
                t.showReading(rendered);
            }
            const seconds = (rendered.samples.length / rendered.sampleRate).toFixed(1);
            const words = new Set(rendered.items.filter(item => /\w/.test(rendered.text.slice(item.from, item.to))).map(item => item.from)).size;
            t._status.textContent = `${words} ${words === 1 ? 'word' : 'words'}, ${seconds} seconds.`;
            return rendered;
        },
        /** The only text error is length: selects what doesn't fit and explains. Returns null, for render to pass on. */
        showTooLong(text, { start, end }) {
            const t = this;
            if (t._text.value !== text) return null;
            t._text.setAttribute('aria-invalid', 'true');
            t._text.setSelectionRange(start, end);
            t._text.focus();
            t._status.textContent = 'That is more than we can say in one go. The selected text is past the limit, so try a shorter part.';
            return null;
        },
        /**
         * Shows the text as it will be read, with a span for each word to highlight while it is spoken, and the IPA
         * transcript.
         */
        showReading(rendered) {
            const t = this;
            const { text, items } = rendered;
            const spans = new Map();
            const parts = [];
            let at = 0;
            const ranges = [...new Map(items.filter(item => item.to > item.from && /\w/.test(text.slice(item.from, item.to)))
                .map(item => [item.from, item])).values()].sort((a, b) => a.from - b.from);
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
            t._reading.replaceChildren(...parts);
            t._words = items.map(item => spans.get(item.from) || null);
            t._ipa.textContent = rendered.transcript;
        },
        async toggleSpeak() {
            const t = this;
            if (t._source) {
                t._source.stop();
                return;
            }
            // The AudioContext has to be made inside the click, for browsers' autoplay rules.
            if (!t._ctx) t._ctx = new AudioContext();
            t._ctx.resume();
            t._speak.disabled = true;
            const rendered = await t.render(t._ctx.sampleRate);
            t._speak.disabled = false;
            if (!rendered || !t._ctx) return;
            t.play(rendered);
        },
        play(rendered) {
            const t = this;
            const buffer = t._ctx.createBuffer(1, rendered.samples.length, rendered.sampleRate);
            buffer.copyToChannel(rendered.samples, 0);
            const source = t._ctx.createBufferSource();
            source.buffer = buffer;
            source.connect(t._ctx.destination);
            source.onended = () => t.stopped(source);
            t._source = source;
            t._startedAt = t._ctx.currentTime;
            source.start();
            t._speak.textContent = 'Stop';
            t._speak.setAttribute('aria-pressed', 'true');
            t.follow(rendered);
        },
        stopped(source) {
            const t = this;
            if (t._source !== source) return;
            t._source = null;
            t._current?.classList.remove('now');
            t._current = null;
            t._speak.textContent = 'Speak';
            t._speak.setAttribute('aria-pressed', 'false');
        },
        /** Highlights the word being spoken. Pauses keep the last word lit. */
        follow(rendered) {
            const t = this;
            const pass = t._pass = (t._pass || 0) + 1;
            const frame = () => {
                if (pass !== t._pass || !t._source || !t._ctx || !t.isConnected) return;
                const sample = (t._ctx.currentTime - t._startedAt) * rendered.sampleRate;
                const index = rendered.items.findIndex(item => sample >= item.start && sample < item.end);
                const word = index >= 0 ? t._words[index] : null;
                if (word && word !== t._current) {
                    t._current?.classList.remove('now');
                    word.classList.add('now');
                    t._current = word;
                }
                requestAnimationFrame(frame);
            };
            requestAnimationFrame(frame);
        },
        async saveWav() {
            const t = this;
            t._save.disabled = true;
            const rendered = await t.render(t._ctx?.sampleRate || DEFAULT_SAMPLE_RATE);
            t._save.disabled = false;
            if (!rendered) return;
            const url = URL.createObjectURL(encodeWav(rendered.samples, rendered.sampleRate));
            const link = document.createElement('a');
            link.href = url;
            link.download = `vocal-bin-speech-${new Date().toISOString().slice(0, 19).replace(/[T:]/g, '-')}.wav`;
            link.click();
            setTimeout(() => URL.revokeObjectURL(url), 10000);
            const seconds = (rendered.samples.length / rendered.sampleRate).toFixed(1);
            t._status.textContent = `Saved ${seconds} seconds as ${link.download}`;
        },
        shutdown() {
            const t = this;
            const source = t._source;
            t._source = null;
            source?.stop();
            t._ctx?.close();
            t._ctx = null;
            t._speak.disabled = false;
            t._speak.textContent = 'Speak';
            t._speak.setAttribute('aria-pressed', 'false');
        },
        shadowTemplate: `
<label class="text"><span>Text</span><textarea rows="6" maxlength="10000" aria-describedby="hint"></textarea></label>
<p id="hint" class="hint">Type or paste English. Numbers, dates, money, and common abbreviations are read as words. Ctrl+Enter speaks.</p>
<div class="row examples" role="group" aria-label="Examples">${exampleButtons}</div>
<div class="row">
<button type="button" class="speak" aria-pressed="false">Speak</button>
<button type="button" class="save">Save WAV</button>
<span class="status" role="status" aria-live="polite"></span>
</div>
<p class="reading" aria-hidden="true"></p>
<details class="pronunciation">
<summary>Pronunciation</summary>
<p class="hint">What the voice says, in IPA. You can copy it into the <a href="/phoneme-lab">Phoneme Lab</a> to change how a word sounds.</p>
<p class="ipa" lang="en-fonipa"></p>
</details>
<fieldset class="presets"><legend>Voice</legend>${presetButtons}</fieldset>
<fieldset class="sliders"><legend>Shape the voice</legend>${sliders}</fieldset>
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
button[aria-pressed="true"] {
background-color:var(--color-primary);
color:var(--color-primary-offset, #fff);
}
button:disabled {
opacity:0.6;
cursor:default;
}
button:focus-visible,
input:focus-visible,
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
.speak,
.save {
min-width:7em;
font-weight:bold;
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
.sliders {
display:grid;
grid-template-columns:repeat(auto-fit, minmax(16em, 1fr));
gap:0.8em 1.5em;
}
.slider {
display:grid;
grid-template-columns:1fr auto;
gap:0.2em;
}
.slider output {
font-size:0.9em;
opacity:0.85;
}
.slider input {
grid-column:1 / -1;
width:100%;
accent-color:var(--color-title);
}
</style>
`
    });
}

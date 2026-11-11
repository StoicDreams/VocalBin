/*!
 * Vocal Bin Phoneme Lab
 * Stage 2 of the Vocal Bin speech synthesizer: type ARPAbet, hear it spoken, and save it as a WAV file.
 * The sound comes from synth/ (Rust, compiled to vocal-synth.wasm), rendered in a Web Worker.
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // Parameter numbers, the same as vocal_synth::param in synth/src/voice.rs.
    const PARAM = { pitch: 0, tract: 1, rd: 2, breath: 3, roughness: 4, volume: 7, rate: 10, intonation: 11 };
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
    const EXAMPLES = [
        ['Hello, world.', 'HH AH0 L OW1 , W ER1 L D .'],
        ['How are you?', 'HH AW1 AA1 R Y UW1 ?'],
        ['Speech synthesis.', 'S P IY1 CH , S IH1 N TH AH0 S AH0 S .'],
        ['Vocal Bin.', 'V OW1 K AH0 L , B IH1 N .'],
        ['She sells sea shells.', 'SH IY1 S EH1 L Z S IY1 SH EH1 L Z .'],
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
    // The worker script loads from a Blob URL, so its request takes the same path as other site files.
    const loadWorkerUrl = once('/vocal-synth-speaker-worker.min.js', response => response.text()
        .then(code => URL.createObjectURL(new Blob([code], { type: 'text/javascript' }))));

    // One worker renders for every phoneme lab on the page.
    let worker = null;
    let nextJob = 1;
    const jobs = new Map();
    async function renderInWorker(job) {
        if (!worker) {
            const [bytes, url] = await Promise.all([loadWasm(), loadWorkerUrl()]);
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
                worker.postMessage({ id: 0, wasm: bytes, settings: [], text: '', sampleRate: DEFAULT_SAMPLE_RATE });
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

    const chartButtons = (list, vowel) => list.map(([symbol, ipa, word]) =>
        `<button type="button" data-symbol="${symbol}${vowel ? '1' : ''}" aria-label="${symbol}, ${ipa} as in ${word}" title="${ipa} as in ${word}"><span class="sym">${symbol}</span><span class="ipa">${ipa}</span></button>`
    ).join('');
    const exampleButtons = EXAMPLES.map(([label], i) => `<button type="button" data-example="${i}">${label}</button>`).join('');
    const presetButtons = Object.keys(PRESETS).map(name => `<button type="button" data-preset="${name}">${name}</button>`).join('');
    const sliders = SLIDERS.map(s =>
        `<label class="slider"><span class="name">${s.label}</span><output aria-hidden="true"></output><input type="range" data-key="${s.key}" aria-label="${s.label}" min="${s.min}" max="${s.max}" step="${s.step}" value="${s.value}"></label>`
    ).join('');

    webui.define("app-phoneme-lab", {
        constructor() {
            const t = this;
            t._values = {};
            t._text = t.template.querySelector('textarea');
            t._speak = t.template.querySelector('.speak');
            t._save = t.template.querySelector('.save');
            t._status = t.template.querySelector('.status');
            t._timeline = t.template.querySelector('.timeline');
            t._inputs = [...t.template.querySelectorAll('input[type=range]')];
            t._text.value = EXAMPLES[0][1];
            t._loop = t.template.querySelector('.loop');
            t._inputs.forEach(input => {
                t.showValue(input);
                input.addEventListener('input', () => {
                    t.showValue(input);
                    t.prepareNextLoop();
                });
            });
            t._text.addEventListener('input', () => {
                t._text.removeAttribute('aria-invalid');
                t.prepareNextLoop();
            });
            t._loop.addEventListener('click', () => {
                t._looping = !t._looping;
                t._loop.setAttribute('aria-pressed', String(t._looping));
            });
            t._text.addEventListener('keydown', ev => {
                if (ev.key === 'Enter' && (ev.ctrlKey || ev.metaKey)) {
                    ev.preventDefault();
                    t.toggleSpeak();
                }
            });
            t.template.querySelectorAll('[data-symbol]').forEach(button => {
                button.addEventListener('click', () => t.insert(button.dataset.symbol));
            });
            t.template.querySelectorAll('[data-example]').forEach(button => {
                button.addEventListener('click', () => {
                    t._text.value = EXAMPLES[Number(button.dataset.example)][1];
                    t._text.removeAttribute('aria-invalid');
                    t.prepareNextLoop();
                });
            });
            t.template.querySelectorAll('[data-preset]').forEach(button => {
                button.addEventListener('click', () => t.applyPreset(button.dataset.preset));
            });
            t._speak.addEventListener('click', () => t.toggleSpeak());
            t._save.addEventListener('click', () => t.saveWav());
        },
        connected() { },
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
            t.prepareNextLoop();
        },
        /**
         * While looping, renders the next pass in the background shortly after a change, so it is ready when the
         * current pass ends. The render is cached, so the next pass picks it up without waiting.
         */
        prepareNextLoop() {
            const t = this;
            if (!t._looping || !t._source || !t._ctx) return;
            clearTimeout(t._prepareTimer);
            const sampleRate = t._ctx.sampleRate;
            t._prepareTimer = setTimeout(() => t.render(sampleRate, true), 150);
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
        },
        /**
         * Renders the text with the current settings, or reuses the last render if nothing changed. A quiet render
         * only fills the cache, leaving the status, text box, and timeline alone.
         */
        async render(sampleRate, quiet = false) {
            const t = this;
            const text = t._text.value;
            if (!text.trim()) {
                if (!quiet) t._status.textContent = 'Type some phonemes first, or pick an example.';
                return null;
            }
            const settings = Object.keys(PARAM).map(key => [PARAM[key], t._values[key]]);
            const key = JSON.stringify([text, settings, sampleRate]);
            if (t._rendered?.key !== key) {
                if (t._pending?.key !== key) {
                    t._pending = { key, job: renderInWorker({ text, settings, sampleRate }) };
                }
                const pending = t._pending;
                if (!quiet) t._status.textContent = 'Rendering…';
                let result;
                try {
                    result = await pending.job;
                } catch (error) {
                    console.error('Phoneme lab rendering failed', error);
                    if (!quiet) t._status.textContent = 'Sorry, the speech could not be rendered in this browser.';
                    return null;
                } finally {
                    if (t._pending === pending) t._pending = null;
                }
                if (result.parseError) {
                    return quiet ? null : t.showParseError(text, result.parseError);
                }
                // A newer render may have finished while this one waited.
                if (t._rendered?.key !== key) t._rendered = { key, text, ...result };
            }
            if (quiet) return t._rendered;
            const rendered = t._rendered;
            if (t._shown !== rendered) {
                t._shown = rendered;
                t.showTimeline(rendered);
            }
            const seconds = (rendered.samples.length / rendered.sampleRate).toFixed(1);
            const count = rendered.items.filter(item => !/^[,.!?]$/.test(rendered.text.slice(item.from, item.to))).length;
            t._status.textContent = `${count} phonemes, ${seconds} seconds.`;
            return rendered;
        },
        /** Selects the text that isn't ARPAbet and explains why. Returns null, for render to pass on. */
        showParseError(text, { start, end }) {
            const t = this;
            t._text.setAttribute('aria-invalid', 'true');
            t._text.setSelectionRange(start, end);
            t._status.textContent = `“${text.slice(start, end)}” isn't an ARPAbet phoneme. Vowels take a stress number (0, 1, or 2), and consonants take none.`;
            return null;
        },
        showTimeline(rendered) {
            const t = this;
            const chips = rendered.items.map(item => {
                const chip = document.createElement('span');
                chip.textContent = rendered.text.slice(item.from, item.to);
                chip.style.flexGrow = String(item.end - item.start);
                return chip;
            });
            t._timeline.replaceChildren(...chips);
            t._chips = chips;
        },
        async toggleSpeak() {
            const t = this;
            if (t._source) {
                t._stopAsked = true;
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
            t._stopAsked = false;
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
        async stopped(source) {
            const t = this;
            if (t._source !== source) return;
            t._chips?.forEach(chip => chip.classList.remove('now'));
            if (t._looping && !t._stopAsked && t._ctx && t.isConnected) {
                // Play again with the current text and settings. Usually already rendered by prepareNextLoop.
                const rendered = await t.render(t._ctx?.sampleRate);
                if (rendered && t._source === source && !t._stopAsked && t._ctx) {
                    t.play(rendered);
                    return;
                }
            }
            if (t._source !== source) return;
            t._source = null;
            t._speak.textContent = 'Speak';
            t._speak.setAttribute('aria-pressed', 'false');
        },
        /** Highlights the phoneme being spoken. */
        follow(rendered) {
            const t = this;
            let current = null;
            // Each pass of a loop follows on its own. An earlier pass's frames stop when a new pass starts.
            const pass = t._pass = (t._pass || 0) + 1;
            const frame = () => {
                if (pass !== t._pass || !t._source || !t._ctx || !t.isConnected) return;
                const sample = (t._ctx.currentTime - t._startedAt) * rendered.sampleRate;
                const index = rendered.items.findIndex(item => sample >= item.start && sample < item.end);
                const chip = index >= 0 ? t._chips[index] : null;
                if (chip !== current) {
                    current?.classList.remove('now');
                    chip?.classList.add('now');
                    current = chip;
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
            link.download = `vocal-bin-phoneme-lab-${new Date().toISOString().slice(0, 19).replace(/[T:]/g, '-')}.wav`;
            link.click();
            setTimeout(() => URL.revokeObjectURL(url), 10000);
            const seconds = (rendered.samples.length / rendered.sampleRate).toFixed(1);
            t._status.textContent = `Saved ${seconds} seconds as ${link.download}`;
        },
        shutdown() {
            const t = this;
            clearTimeout(t._prepareTimer);
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
<label class="text"><span>Phonemes (ARPAbet)</span><textarea rows="3" maxlength="4000" spellcheck="false" autocapitalize="characters" aria-describedby="hint"></textarea></label>
<p id="hint" class="hint">Separate phonemes with spaces. Vowels take a stress number: 1 stressed, 2 secondary, 0 unstressed. A comma pauses, and a period or question mark ends a sentence. Ctrl+Enter speaks. Loop repeats the speech and picks up your changes on each pass.</p>
<div class="row examples" role="group" aria-label="Examples">${exampleButtons}</div>
<div class="row">
<button type="button" class="speak" aria-pressed="false">Speak</button>
<button type="button" class="loop" aria-pressed="false">Loop</button>
<button type="button" class="save">Save WAV</button>
<span class="status" role="status" aria-live="polite"></span>
</div>
<div class="timeline" aria-hidden="true"></div>
<details class="chart">
<summary>Phoneme chart</summary>
<p class="hint">Each button adds its phoneme at the cursor. Vowels are added stressed.</p>
<fieldset><legend>Vowels</legend>${chartButtons(VOWELS, true)}</fieldset>
<fieldset><legend>Consonants</legend>${chartButtons(CONSONANTS, false)}</fieldset>
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
summary:focus-visible {
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

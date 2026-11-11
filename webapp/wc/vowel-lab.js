/*!
 * Vocal Bin Vowel Lab
 * Stage 1 of the Vocal Bin speech synthesizer: voice a vowel, shape it with sliders, and record it as a WAV file.
 * The sound comes from synth/ (Rust, compiled to vocal-synth.wasm), run in an AudioWorklet.
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // Parameter numbers, the same as vocal_synth::param in synth/src/synth.rs.
    const PARAM = { pitch: 0, tract: 1, rd: 2, breath: 3, roughness: 4, vibrato: 5, glide: 6, volume: 7, vowel: 8, voicing: 9 };
    // The same order as VOWELS in synth/src/vowels.rs.
    const VOWELS = [
        ['i', 'heed'], ['ɪ', 'hid'], ['e', 'hayed'], ['ɛ', 'head'], ['æ', 'had'], ['ʌ', 'hud'],
        ['ɑ', 'hod'], ['ɔ', 'hawed'], ['o', 'hoed'], ['ʊ', 'hood'], ['u', "who'd"], ['ɝ', 'heard'],
    ];
    const DEFAULT_VOWEL = 6;
    const percent = v => `${Math.round(v * 100)}%`;
    const quality = v => `${v < 0.8 ? 'pressed' : v > 1.6 ? 'lax' : 'modal'} (${v.toFixed(2)})`;
    const SLIDERS = [
        { key: 'pitch', label: 'Pitch', min: 50, max: 400, step: 1, value: 110, show: v => `${v} Hz` },
        { key: 'tract', label: 'Vocal tract length', min: 10, max: 22, step: 0.1, value: 17.5, show: v => `${v.toFixed(1)} cm` },
        { key: 'rd', label: 'Voice quality', min: 0.3, max: 2.7, step: 0.05, value: 1, show: quality },
        { key: 'breath', label: 'Breathiness', min: 0, max: 1, step: 0.01, value: 0.1, show: percent },
        { key: 'roughness', label: 'Roughness', min: 0, max: 1, step: 0.01, value: 0.15, show: percent },
        { key: 'vibrato', label: 'Vibrato', min: 0, max: 2, step: 0.05, value: 0, show: v => `${v.toFixed(2)} semitones` },
        { key: 'glide', label: 'Glide time', min: 10, max: 1000, step: 10, value: 120, show: v => `${v} ms` },
        { key: 'volume', label: 'Volume', min: 0, max: 1, step: 0.01, value: 0.8, show: percent },
    ];
    const PRESETS = {
        Man: { pitch: 110, tract: 17.5, rd: 1, breath: 0.1 },
        Woman: { pitch: 210, tract: 14.8, rd: 1.4, breath: 0.2 },
        Child: { pitch: 290, tract: 12.5, rd: 1.3, breath: 0.15 },
    };
    // Recording stops on its own after this long, to keep memory use reasonable.
    const MAX_RECORD_SECONDS = 300;

    let wasmBytes = null;
    /** The synthesizer's wasm, fetched once per visit. */
    function loadWasm() {
        if (!wasmBytes) {
            wasmBytes = fetch('/vocal-synth.wasm').then(response => {
                if (!response.ok) throw new Error(`vocal-synth.wasm: ${response.status}`);
                return response.arrayBuffer();
            });
            wasmBytes.catch(() => { wasmBytes = null; });
        }
        return wasmBytes;
    }

    let workletUrl = null;
    /**
     * The worklet script as a Blob URL, made once per visit. Fetching it from the page, rather than letting
     * addModule request it, keeps it on the same path as other files (the service worker's cache, and test routing).
     */
    function loadWorklet() {
        if (!workletUrl) {
            workletUrl = fetch('/vocal-synth-worklet.min.js').then(response => {
                if (!response.ok) throw new Error(`vocal-synth-worklet.min.js: ${response.status}`);
                return response.text();
            }).then(code => URL.createObjectURL(new Blob([code], { type: 'text/javascript' })));
            workletUrl.catch(() => { workletUrl = null; });
        }
        return workletUrl;
    }

    /** 16-bit mono WAV from chunks of samples between -1 and 1. */
    function encodeWav(chunks, sampleRate) {
        const length = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
        const view = new DataView(new ArrayBuffer(44 + length * 2));
        const text = (offset, value) => {
            for (let i = 0; i < value.length; i++) view.setUint8(offset + i, value.charCodeAt(i));
        };
        text(0, 'RIFF');
        view.setUint32(4, 36 + length * 2, true);
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
        view.setUint32(40, length * 2, true);
        let offset = 44;
        for (const chunk of chunks) {
            for (let i = 0; i < chunk.length; i++) {
                const s = Math.max(-1, Math.min(1, chunk[i]));
                view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7fff, true);
                offset += 2;
            }
        }
        return new Blob([view.buffer], { type: 'audio/wav' });
    }

    function clock(seconds) {
        const s = Math.floor(seconds);
        return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
    }

    const vowelButtons = VOWELS.map(([symbol, word], i) =>
        `<button type="button" data-vowel="${i}" aria-pressed="${i === DEFAULT_VOWEL}" aria-label="${symbol} as in ${word}"><span class="ipa">${symbol}</span><span class="word">${word}</span></button>`
    ).join('');
    const presetButtons = Object.keys(PRESETS).map(name => `<button type="button" data-preset="${name}">${name}</button>`).join('');
    const sliders = SLIDERS.map(s =>
        `<label class="slider"><span class="name">${s.label}</span><output aria-hidden="true"></output><input type="range" data-key="${s.key}" aria-label="${s.label}" min="${s.min}" max="${s.max}" step="${s.step}" value="${s.value}"></label>`
    ).join('');

    webui.define("app-vowel-lab", {
        constructor() {
            const t = this;
            t._values = {};
            t._play = t.template.querySelector('.play');
            t._record = t.template.querySelector('.record');
            t._status = t.template.querySelector('.status');
            t._canvas = t.template.querySelector('canvas');
            t._vowels = [...t.template.querySelectorAll('[data-vowel]')];
            t._inputs = [...t.template.querySelectorAll('input[type=range]')];
            t._values.vowel = DEFAULT_VOWEL;
            t._values.voicing = 0;
            t._inputs.forEach(input => {
                t.showValue(input);
                input.addEventListener('input', () => {
                    t.showValue(input);
                    t.send(input.dataset.key, Number(input.value));
                });
            });
            t._vowels.forEach(button => {
                button.addEventListener('click', () => t.setVowel(Number(button.dataset.vowel)));
            });
            t.template.querySelectorAll('[data-preset]').forEach(button => {
                button.addEventListener('click', () => t.applyPreset(button.dataset.preset));
            });
            t._play.addEventListener('click', () => t.togglePlay());
            t._record.addEventListener('click', () => t.toggleRecord());
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
        send(key, value) {
            const t = this;
            t._values[key] = value;
            t._node?.port.postMessage({ type: 'set', id: PARAM[key], value });
        },
        setVowel(index) {
            const t = this;
            t._vowels.forEach(button => button.setAttribute('aria-pressed', String(Number(button.dataset.vowel) === index)));
            t.send('vowel', index);
        },
        applyPreset(name) {
            const t = this;
            const preset = PRESETS[name];
            if (!preset) return;
            t._inputs.forEach(input => {
                const key = input.dataset.key;
                if (key in preset) {
                    input.value = preset[key];
                    t.showValue(input);
                    t.send(key, preset[key]);
                }
            });
        },
        /** Starts the audio graph on the first press. The AudioContext has to be made inside the click. */
        startAudio() {
            const t = this;
            if (t._ready) return t._ready;
            t._ctx = new AudioContext();
            t._ready = (async () => {
                const ctx = t._ctx;
                const [bytes] = await Promise.all([loadWasm(), loadWorklet().then(url => ctx.audioWorklet.addModule(url))]);
                const node = new AudioWorkletNode(t._ctx, 'vocal-synth', {
                    numberOfInputs: 0,
                    outputChannelCount: [1],
                    processorOptions: { wasm: bytes },
                });
                node.port.onmessage = ev => t.receive(ev.data);
                t._analyser = t._ctx.createAnalyser();
                t._analyser.fftSize = 2048;
                node.connect(t._analyser);
                t._analyser.connect(t._ctx.destination);
                t._node = node;
                Object.keys(t._values).forEach(key => t.send(key, t._values[key]));
            })();
            t._ready.catch(error => {
                console.error('Vowel lab audio failed to start', error);
                t._status.textContent = 'Sorry, audio could not start in this browser.';
                t._ctx?.close();
                t._ctx = null;
                t._ready = null;
            });
            return t._ready;
        },
        async togglePlay() {
            const t = this;
            const playing = t._values.voicing !== 1;
            t._play.setAttribute('aria-pressed', String(playing));
            t._play.textContent = playing ? 'Stop' : 'Play';
            if (playing) {
                // Set before loading, so the synth starts voiced and a second press while loading counts as Stop.
                t._values.voicing = 1;
                clearTimeout(t._suspendTimer);
                const ready = t.startAudio();
                t._ctx.resume();
                try { await ready; } catch { return t.resetPlay(); }
                if (t._values.voicing !== 1) return;
                t.send('voicing', 1);
                t.draw();
            } else {
                t.send('voicing', 0);
                t.suspendSoon();
            }
        },
        resetPlay() {
            const t = this;
            t._values.voicing = 0;
            t._play.setAttribute('aria-pressed', 'false');
            t._play.textContent = 'Play';
        },
        /** Pauses the audio thread once the voice has faded out, unless something still needs it. */
        suspendSoon() {
            const t = this;
            clearTimeout(t._suspendTimer);
            t._suspendTimer = setTimeout(() => {
                if (t._values.voicing !== 1 && !t._recording) t._ctx?.suspend();
            }, 200);
        },
        async toggleRecord() {
            const t = this;
            if (t._recording) {
                t._record.disabled = true;
                t._node.port.postMessage({ type: 'stop-record' });
                return;
            }
            clearTimeout(t._suspendTimer);
            const ready = t.startAudio();
            t._ctx.resume();
            try { await ready; } catch { return; }
            t._chunks = [];
            t._recorded = 0;
            t._recording = true;
            t._record.setAttribute('aria-pressed', 'true');
            t._record.textContent = 'Stop recording';
            t._status.textContent = 'Recording 0:00';
            t._node.port.postMessage({ type: 'record' });
        },
        receive(msg) {
            const t = this;
            if (msg.type !== 'audio' || !t._recording) return;
            t._chunks.push(msg.samples);
            t._recorded += msg.samples.length;
            const seconds = t._recorded / t._ctx.sampleRate;
            if (msg.last) {
                t.finishRecording();
            } else {
                t._status.textContent = `Recording ${clock(seconds)}`;
                if (seconds >= MAX_RECORD_SECONDS && !t._record.disabled) {
                    t._record.disabled = true;
                    t._node.port.postMessage({ type: 'stop-record' });
                }
            }
        },
        finishRecording() {
            const t = this;
            t._recording = false;
            t._record.disabled = false;
            t._record.setAttribute('aria-pressed', 'false');
            t._record.textContent = 'Record';
            const seconds = t._recorded / t._ctx.sampleRate;
            const blob = encodeWav(t._chunks, t._ctx.sampleRate);
            t._chunks = [];
            const url = URL.createObjectURL(blob);
            const link = document.createElement('a');
            link.href = url;
            link.download = `vocal-bin-vowel-lab-${new Date().toISOString().slice(0, 19).replace(/[T:]/g, '-')}.wav`;
            link.click();
            setTimeout(() => URL.revokeObjectURL(url), 10000);
            t._status.textContent = `Saved ${clock(seconds)} as ${link.download}`;
            t.suspendSoon();
        },
        /** Draws the waveform while the voice is on. */
        draw() {
            const t = this;
            if (t._drawing || !t._analyser) return;
            t._drawing = true;
            const canvas = t._canvas;
            const context = canvas.getContext('2d');
            const data = new Float32Array(t._analyser.fftSize);
            const color = getComputedStyle(t).getPropertyValue('--color-title').trim() || '#d10694';
            const frame = () => {
                const { width, height } = canvas;
                context.clearRect(0, 0, width, height);
                if (t._values.voicing !== 1 || !t.isConnected) {
                    t._drawing = false;
                    return;
                }
                t._analyser.getFloatTimeDomainData(data);
                context.strokeStyle = color;
                context.lineWidth = 2;
                context.beginPath();
                for (let i = 0; i < data.length; i++) {
                    const x = i / (data.length - 1) * width;
                    const y = (0.5 - data[i] * 0.45) * height;
                    if (i === 0) context.moveTo(x, y);
                    else context.lineTo(x, y);
                }
                context.stroke();
                requestAnimationFrame(frame);
            };
            requestAnimationFrame(frame);
        },
        shutdown() {
            const t = this;
            clearTimeout(t._suspendTimer);
            t._ctx?.close();
            t._ctx = null;
            t._node = null;
            t._analyser = null;
            t._ready = null;
            t._recording = false;
            t._record.disabled = false;
            t._record.setAttribute('aria-pressed', 'false');
            t._record.textContent = 'Record';
            t.resetPlay();
        },
        shadowTemplate: `
<div class="bar">
<button type="button" class="play" aria-pressed="false">Play</button>
<button type="button" class="record" aria-pressed="false">Record</button>
<span class="status" role="status" aria-live="polite"></span>
</div>
<canvas width="1000" height="160" aria-hidden="true"></canvas>
<fieldset class="vowels"><legend>Vowel</legend>${vowelButtons}</fieldset>
<fieldset class="presets"><legend>Voice</legend>${presetButtons}</fieldset>
<fieldset class="sliders"><legend>Shape the voice</legend>${sliders}</fieldset>
<style type="text/css">
:host {
display:grid;
gap:var(--padding, 1em);
}
.bar {
display:flex;
flex-wrap:wrap;
align-items:center;
gap:var(--padding, 1em);
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
input:focus-visible {
outline:2px solid var(--color-focus, currentColor);
outline-offset:2px;
}
.play,
.record {
min-width:7em;
font-weight:bold;
}
canvas {
width:100%;
height:8em;
border-radius:0.4em;
background-color:var(--color-black, #101010);
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
.vowels button {
display:grid;
justify-items:center;
min-width:4.5em;
padding:0.3em 0.6em;
}
.ipa {
font-size:1.4em;
line-height:1.2;
}
.word {
font-size:0.8em;
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

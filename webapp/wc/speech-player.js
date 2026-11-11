/*!
 * Vocal Bin Speech Player
 * The Speak, Loop, and Save WAV buttons and status for Vocal Bin's speech synthesizer, used by the Phoneme Lab and
 * Text to Speech. It renders in a Web Worker with synth/ (Rust, compiled to vocal-synth.wasm), plays the speech, and
 * says which part of the text is being spoken.
 *
 * Long text is split at sentence ends into parts of at most PART characters (the first part shorter, so speech
 * starts soon). Playback starts when the first part is ready, and each next part renders while the one before plays.
 * Save WAV renders every part and joins them.
 *
 * <app-speech-player file-prefix="vocal-bin-speech" busy-text="Reading…" empty-text="Type some text first." loop dictionary></app-speech-player>
 *
 * - `loop` shows a Loop toggle, which replays until stopped, picking up changes on each pass.
 * - `dictionary` starts fetching the pronouncing dictionary when the player connects, for ordinary text.
 *
 * The page using it sets these properties:
 * - `getJob()` returns `{ text, notation, settings }` (notation 'arpabet', 'ipa', or 'text'), or null when there is
 *   nothing to say, and the player shows `empty-text`.
 * - `describe(reading)` returns the status after rendering: `reading` has `text`, `items`, `transcript`, `seconds`,
 *   and `done` (false while later parts are still rendering).
 * - `parseErrorMessage(job, { start, end })` returns the status for text that can't be read, and may select it.
 *
 * Events: `reading` (detail: the reading so far, after each part renders) and `item` (detail: the index in
 * `reading.items` being spoken, or -1 when nothing is).
 *
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    const DEFAULT_SAMPLE_RATE = 44100;
    /** Characters in the first part, and in each part after it. Each part stays well inside the synthesizer's limit
     * of 2000 sounds (MAX_TOKENS in synth/src/phonemes.rs). */
    const FIRST_PART = 300;
    const PART = 1000;
    /** Renders kept for reuse, so Speak after Save WAV, or a loop pass, doesn't render again. */
    const CACHE_SIZE = 64;

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

    // One worker renders for every player on the page, one job at a time in the order they are sent.
    let worker = null;
    let dictionarySent = null;
    let nextJob = 1;
    const jobs = new Map();
    async function startWorker() {
        if (worker) return;
        const [bytes, url] = await Promise.all([loadWasm(), loadWorkerUrl()]);
        if (worker) return;
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
            dictionarySent = null;
        };
        worker.postMessage({ id: 0, wasm: bytes, settings: [], text: '', sampleRate: DEFAULT_SAMPLE_RATE });
    }
    /** Sends the dictionary to the worker once, before the first text job. Without it, text is read by spelling. */
    function sendDictionary() {
        dictionarySent ??= loadDictionary()
            .then(dictionary => worker?.postMessage({ id: 0, dictionary, settings: [], text: '', notation: 'text', sampleRate: DEFAULT_SAMPLE_RATE }))
            .catch(error => console.warn('Vocal Bin: the pronouncing dictionary did not load, so words come from spelling rules.', error));
        return dictionarySent;
    }
    async function renderInWorker(job) {
        await startWorker();
        if (job.notation === 'text') await sendDictionary();
        const id = nextJob++;
        return new Promise((resolve, reject) => {
            jobs.set(id, { resolve, reject });
            worker.postMessage({ id, ...job });
        });
    }

    /** Where a sentence ends, so a part can end there. Not after a short capitalized word or one letter ("Dr.", "U.S."). */
    const SENTENCE_END = /(?<![\s.][A-Z][a-z]{0,2}|\b[A-Za-z])[.!?…]+["'”’)\]]*\s+/g;
    /** The end of the last match of `pattern` in `text`, or 0. */
    function lastEnd(text, pattern) {
        let end = 0;
        for (const match of text.matchAll(pattern)) end = match.index + match[0].length;
        return end;
    }
    /** Splits text into parts at sentence ends, or failing that at a comma or space: [{ from, to }]. */
    function split(text) {
        const parts = [];
        let from = 0;
        while (from < text.length) {
            const limit = parts.length ? PART : FIRST_PART;
            if (text.length - from <= limit) {
                parts.push({ from, to: text.length });
                break;
            }
            const window = text.slice(from, from + limit);
            const cut = lastEnd(window, SENTENCE_END) || lastEnd(window, /[,;:]\s+/g) || lastEnd(window, /\s+/g) || limit;
            parts.push({ from, to: from + cut });
            from += cut;
        }
        return parts.filter(part => text.slice(part.from, part.to).trim());
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

    webui.define("app-speech-player", {
        attr: ['file-prefix', 'busy-text', 'empty-text'],
        flags: ['loop', 'dictionary'],
        constructor() {
            const t = this;
            t._cache = new Map();
            t._sources = [];
            // t.template is emptied once it moves into the shadow root, so find everything needed later now.
            t._speak = t.template.querySelector('.speak');
            t._loopButton = t.template.querySelector('.loop');
            t._save = t.template.querySelector('.save');
            t._status = t.template.querySelector('.status');
            t._speak.addEventListener('click', () => t.toggle());
            t._save.addEventListener('click', () => t.saveWav());
            t._loopButton.addEventListener('click', () => {
                t._looping = !t._looping;
                t._loopButton.setAttribute('aria-pressed', String(t._looping));
            });
        },
        attrChanged(property) {
            if (property === 'loop') this._loopButton.hidden = !this.loop;
        },
        connected() {
            const t = this;
            t._loopButton.hidden = !t.loop;
            // Start fetching the dictionary now, so the first Speak doesn't wait for it.
            if (t.dictionary) loadDictionary().catch(() => { });
        },
        disconnected() {
            const t = this;
            // Moving the element fires disconnected then reconnected. Only shut down if it has really left the page.
            setTimeout(() => {
                if (!t.isConnected) t.shutdown();
            }, 0);
        },
        /** Shows a message in the status line, for the page using the player. */
        setStatus(text) {
            this._status.textContent = text;
        },
        /** The page calls this when its text or settings change, so a loop renders the next pass ahead of time. */
        changed() {
            const t = this;
            if (!t._looping || !t._playing || !t._ctx) return;
            clearTimeout(t._prepareTimer);
            t._prepareTimer = setTimeout(() => {
                const job = t.getJob?.();
                if (job && t._ctx) t.makePass(job, t._ctx.sampleRate).part(0).catch(() => { });
            }, 150);
        },
        /** The job to say, or null with the empty message shown. */
        job() {
            const t = this;
            const job = t.getJob?.();
            if (!job || !job.text.trim()) {
                t._status.textContent = t.emptyText || 'Type something to say first.';
                return null;
            }
            return job;
        },
        /**
         * Splits a job into parts. `part(i)` renders part i, once, and resolves with its samples and with items and a
         * transcript placed in the whole text, or with a parse error placed in the whole text.
         */
        makePass(job, sampleRate) {
            const t = this;
            const parts = split(job.text);
            const results = [];
            const part = i => results[i] ??= t.renderPart(job, parts[i], sampleRate);
            return { job, parts, part, sampleRate, done: [] };
        },
        async renderPart(job, { from, to }, sampleRate) {
            const t = this;
            const text = job.text.slice(from, to);
            const key = JSON.stringify([text, job.notation, job.settings, sampleRate]);
            let result = t._cache.get(key);
            if (!result) {
                result = await renderInWorker({ text, notation: job.notation, settings: job.settings, sampleRate });
                t._cache.set(key, result);
                if (t._cache.size > CACHE_SIZE) t._cache.delete(t._cache.keys().next().value);
            }
            if (result.parseError) {
                return { parseError: { start: result.parseError.start + from, end: result.parseError.end + from } };
            }
            return {
                samples: result.samples,
                sampleRate: result.sampleRate,
                transcript: result.transcript || '',
                items: result.items.map(item => ({ ...item, from: item.from + from, to: item.to + from })),
            };
        },
        /**
         * Waits for part i and adds it to the pass's reading, showing why if it failed. Returns the part, or null.
         * `busy` shows the busy text while waiting. A pass that was stopped meanwhile changes nothing.
         */
        async settle(pass, i, busy) {
            const t = this;
            let rendering = true;
            if (busy) queueMicrotask(() => { if (rendering) t._status.textContent = t.busyText || 'Rendering…'; });
            let result;
            try {
                result = await pass.part(i);
            } catch (error) {
                console.error('Vocal Bin speech rendering failed', error);
                t._status.textContent = 'Sorry, the speech could not be rendered in this browser.';
                return null;
            } finally {
                rendering = false;
            }
            if (pass.stopped) return null;
            if (result.parseError) {
                t._status.textContent = t.parseErrorMessage?.(pass.job, result.parseError) || 'Some of that text could not be read.';
                return null;
            }
            if (!pass.done[i]) {
                pass.done[i] = result;
                t.showReading(pass);
            }
            return result;
        },
        /** Joins the parts rendered so far, in order, and tells the page. */
        showReading(pass) {
            const t = this;
            const items = [];
            const transcripts = [];
            let offset = 0;
            let count = 0;
            for (const part of pass.done) {
                if (!part) break;
                part.items.forEach(item => items.push({ ...item, start: item.start + offset, end: item.end + offset }));
                if (part.transcript) transcripts.push(part.transcript);
                offset += part.samples.length;
                count++;
            }
            const reading = {
                text: pass.job.text,
                items,
                transcript: transcripts.join(' '),
                seconds: offset / pass.sampleRate,
                done: count === pass.parts.length,
            };
            pass.reading = reading;
            t.dispatchEvent(new CustomEvent('reading', { detail: reading }));
            t._status.textContent = t.describe?.(reading) ?? '';
        },
        async toggle() {
            const t = this;
            if (t._playing) {
                t._stopAsked = true;
                t.stop();
                return;
            }
            const job = t.job();
            if (!job) return;
            // The AudioContext has to be made inside the click, for browsers' autoplay rules.
            if (!t._ctx) t._ctx = new AudioContext();
            t._ctx.resume();
            t._speak.disabled = true;
            const pass = t.makePass(job, t._ctx.sampleRate);
            const first = await t.settle(pass, 0, true);
            t._speak.disabled = false;
            if (!first || !t._ctx || t._playing) return;
            t._stopAsked = false;
            t.play(pass);
        },
        /** Plays a pass, starting each part where the last ends, rendering each next part while one plays. */
        async play(pass) {
            const t = this;
            const run = t._run = (t._run || 0) + 1;
            t._pass = pass;
            t.setPlaying(true);
            t._schedule = [];
            let when = t._ctx.currentTime + 0.02;
            let base = 0;
            let last = null;
            for (let i = 0; i < pass.parts.length; i++) {
                const part = await t.settle(pass, i, false);
                if (run !== t._run || !t._ctx) return;
                if (!part) break;
                if (i + 1 < pass.parts.length) pass.part(i + 1).catch(() => { });
                // A part that took longer than the one before it played starts now, after a gap.
                when = Math.max(when, t._ctx.currentTime + 0.02);
                const buffer = t._ctx.createBuffer(1, Math.max(1, part.samples.length), part.sampleRate);
                buffer.copyToChannel(part.samples, 0);
                const source = t._ctx.createBufferSource();
                source.buffer = buffer;
                source.connect(t._ctx.destination);
                source.start(when);
                t._sources.push(source);
                t._schedule.push({ start: when, sampleRate: part.sampleRate, items: part.items, base });
                base += part.items.length;
                when += part.samples.length / part.sampleRate;
                last = source;
                if (i === 0) t.follow(run);
            }
            if (!last) {
                t.finished(run);
                return;
            }
            last.onended = () => t.finished(run);
        },
        async finished(run) {
            const t = this;
            if (run !== t._run) return;
            t._sources = [];
            t.dispatchEvent(new CustomEvent('item', { detail: -1 }));
            if (t._looping && !t._stopAsked && t._ctx && t.isConnected) {
                // Again with the current text and settings, usually already rendered by changed().
                const job = t.getJob?.();
                if (job?.text.trim()) {
                    const pass = t.makePass(job, t._ctx.sampleRate);
                    const first = await t.settle(pass, 0, false);
                    if (first && run === t._run && !t._stopAsked && t._ctx) {
                        t.play(pass);
                        return;
                    }
                }
            }
            if (run === t._run) t.setPlaying(false);
        },
        stop() {
            const t = this;
            t._run = (t._run || 0) + 1;
            if (t._pass) {
                t._pass.stopped = true;
                if (!t._pass.reading?.done) t._status.textContent = 'Stopped.';
                t._pass = null;
            }
            clearTimeout(t._prepareTimer);
            t._sources.forEach(source => {
                source.onended = null;
                try { source.stop(); } catch { }
            });
            t._sources = [];
            t.dispatchEvent(new CustomEvent('item', { detail: -1 }));
            t.setPlaying(false);
        },
        setPlaying(playing) {
            const t = this;
            t._playing = playing;
            t._speak.textContent = playing ? 'Stop' : 'Speak';
            t._speak.setAttribute('aria-pressed', String(playing));
        },
        /** Tells the page which item is being spoken, each frame while playing. */
        follow(run) {
            const t = this;
            let current = -1;
            const frame = () => {
                if (run !== t._run || !t._playing || !t._ctx || !t.isConnected) return;
                const now = t._ctx.currentTime;
                const part = t._schedule.findLast(p => now >= p.start);
                let index = -1;
                if (part) {
                    const sample = (now - part.start) * part.sampleRate;
                    const i = part.items.findIndex(item => sample >= item.start && sample < item.end);
                    if (i >= 0) index = part.base + i;
                }
                if (index >= 0 && index !== current) {
                    current = index;
                    t.dispatchEvent(new CustomEvent('item', { detail: index }));
                }
                requestAnimationFrame(frame);
            };
            requestAnimationFrame(frame);
        },
        async saveWav() {
            const t = this;
            const job = t.job();
            if (!job) return;
            t._save.disabled = true;
            try {
                const pass = t.makePass(job, t._ctx?.sampleRate || DEFAULT_SAMPLE_RATE);
                const parts = [];
                for (let i = 0; i < pass.parts.length; i++) {
                    if (i + 1 < pass.parts.length) pass.part(i + 1).catch(() => { });
                    const part = await t.settle(pass, i, true);
                    if (!part) return;
                    parts.push(part);
                }
                const samples = new Float32Array(parts.reduce((sum, part) => sum + part.samples.length, 0));
                let offset = 0;
                parts.forEach(part => {
                    samples.set(part.samples, offset);
                    offset += part.samples.length;
                });
                const url = URL.createObjectURL(encodeWav(samples, pass.sampleRate));
                const link = document.createElement('a');
                link.href = url;
                link.download = `${t.filePrefix || 'vocal-bin'}-${new Date().toISOString().slice(0, 19).replace(/[T:]/g, '-')}.wav`;
                link.click();
                setTimeout(() => URL.revokeObjectURL(url), 10000);
                t._status.textContent = `Saved ${(samples.length / pass.sampleRate).toFixed(1)} seconds as ${link.download}`;
            } finally {
                t._save.disabled = false;
            }
        },
        shutdown() {
            const t = this;
            t.stop();
            t._ctx?.close();
            t._ctx = null;
            t._speak.disabled = false;
        },
        shadowTemplate: `
<button type="button" class="speak" aria-pressed="false">Speak</button>
<button type="button" class="loop" aria-pressed="false" hidden>Loop</button>
<button type="button" class="save">Save WAV</button>
<span class="status" role="status" aria-live="polite"></span>
<style type="text/css">
:host {
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
button:focus-visible {
outline:2px solid var(--color-focus, currentColor);
outline-offset:2px;
}
[hidden] {
display:none !important;
}
.speak,
.save {
min-width:7em;
font-weight:bold;
}
</style>
`
    });
}

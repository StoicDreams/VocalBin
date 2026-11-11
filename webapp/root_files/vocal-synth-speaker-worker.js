/*!
 * Vocal Bin Speaker Worker
 * Renders ARPAbet text with the Rust speech synthesizer (vocal-synth.wasm), off the page's main thread.
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    const BLOCK = 8192;
    let wasm = null;

    /** Copies UTF-8 text into wasm memory and runs `use` with its pointer and length. */
    function withText(text, use) {
        const bytes = new TextEncoder().encode(text);
        const pointer = wasm.vb_alloc(bytes.length);
        new Uint8Array(wasm.memory.buffer, pointer, bytes.length).set(bytes);
        try {
            return use(pointer, bytes.length, bytes);
        } finally {
            wasm.vb_dealloc(pointer, bytes.length);
        }
    }

    /** Converts a byte offset in `bytes` to a string index, since the page works in characters. */
    function charIndex(bytes, offset) {
        return new TextDecoder().decode(bytes.subarray(0, offset)).length;
    }

    function render(job) {
        const speaker = wasm.vb_speaker_new(job.sampleRate);
        try {
            for (const [id, value] of job.settings) {
                wasm.vb_speaker_set(speaker, id, value);
            }
            const error = withText(job.text, (pointer, length, bytes) => {
                const at = wasm.vb_speaker_speak(speaker, pointer, length);
                if (at < 0) return null;
                const end = at + wasm.vb_speaker_error_len();
                return { start: charIndex(bytes, at), end: charIndex(bytes, end) };
            });
            if (error) return { id: job.id, parseError: error };

            const bytes = new TextEncoder().encode(job.text);
            const items = [];
            for (let i = 0; i < wasm.vb_speaker_items(speaker); i++) {
                const offset = wasm.vb_speaker_item(speaker, i, 2);
                items.push({
                    start: wasm.vb_speaker_item(speaker, i, 0),
                    end: wasm.vb_speaker_item(speaker, i, 1),
                    from: charIndex(bytes, offset),
                    to: charIndex(bytes, offset + wasm.vb_speaker_item(speaker, i, 3)),
                });
            }
            const length = wasm.vb_speaker_length(speaker);
            const samples = new Float32Array(length);
            for (let done = 0; done < length; done += BLOCK) {
                const frames = Math.min(BLOCK, length - done);
                const pointer = wasm.vb_speaker_render(speaker, frames);
                // Rendering can grow the wasm memory, so view it fresh each time.
                samples.set(new Float32Array(wasm.memory.buffer, pointer, frames), done);
            }
            return { id: job.id, samples, items, sampleRate: job.sampleRate };
        } finally {
            wasm.vb_speaker_free(speaker);
        }
    }

    self.onmessage = ev => {
        const job = ev.data;
        try {
            if (!wasm) {
                wasm = new WebAssembly.Instance(new WebAssembly.Module(job.wasm), {}).exports;
            }
            const result = render(job);
            self.postMessage(result, result.samples ? [result.samples.buffer] : []);
        } catch (error) {
            self.postMessage({ id: job.id, error: String(error) });
        }
    };
}

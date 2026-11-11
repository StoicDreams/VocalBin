/*!
 * Vocal Bin Speaker Worker
 * Renders ARPAbet, IPA, or ordinary English text with the Rust speech synthesizer (vocal-synth.wasm), off the page's
 * main thread. Ordinary text is read with the pronouncing dictionary (vocal-synth-en-us.dict) once a job sends it.
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    const BLOCK = 8192;
    let wasm = null;
    // The dictionary in wasm memory, from vb_dictionary_load. 0 until a job sends one, and then text is read with
    // spelling rules alone.
    let dictionary = 0;
    // The notation numbers vb_speaker_speak takes. Text goes to vb_speaker_speak_text instead.
    const NOTATIONS = { arpabet: 0, ipa: 1 };

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
                const at = job.notation === 'text'
                    ? wasm.vb_speaker_speak_text(speaker, dictionary, pointer, length)
                    : wasm.vb_speaker_speak(speaker, pointer, length, NOTATIONS[job.notation] ?? NOTATIONS.arpabet);
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
            const transcript = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, wasm.vb_speaker_transcript(speaker), wasm.vb_speaker_transcript_len(speaker)));
            const length = wasm.vb_speaker_length(speaker);
            const samples = new Float32Array(length);
            for (let done = 0; done < length; done += BLOCK) {
                const frames = Math.min(BLOCK, length - done);
                const pointer = wasm.vb_speaker_render(speaker, frames);
                // Rendering can grow the wasm memory, so view it fresh each time.
                samples.set(new Float32Array(wasm.memory.buffer, pointer, frames), done);
            }
            return { id: job.id, samples, items, transcript, sampleRate: job.sampleRate };
        } finally {
            wasm.vb_speaker_free(speaker);
        }
    }

    /** Reads the dictionary file into wasm memory, replacing any loaded before. */
    function loadDictionary(bytes) {
        if (dictionary) wasm.vb_dictionary_free(dictionary);
        dictionary = 0;
        const data = new Uint8Array(bytes);
        const pointer = wasm.vb_alloc(data.length);
        new Uint8Array(wasm.memory.buffer, pointer, data.length).set(data);
        try {
            dictionary = wasm.vb_dictionary_load(pointer, data.length);
        } finally {
            wasm.vb_dealloc(pointer, data.length);
        }
        if (!dictionary) console.warn('Vocal Bin: the pronouncing dictionary could not be read, so words come from spelling rules.');
    }

    self.onmessage = ev => {
        const job = ev.data;
        try {
            if (!wasm) {
                wasm = new WebAssembly.Instance(new WebAssembly.Module(job.wasm), {}).exports;
            }
            if (job.dictionary) loadDictionary(job.dictionary);
            const result = render(job);
            self.postMessage(result, result.samples ? [result.samples.buffer] : []);
        } catch (error) {
            self.postMessage({ id: job.id, error: String(error) });
        }
    };
}

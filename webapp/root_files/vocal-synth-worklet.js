/*!
 * Vocal Bin Synth Worklet
 * Runs the Rust speech synthesizer (vocal-synth.wasm) on the audio thread, so changes are heard while a slider moves.
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // Recorded audio is sent to the page in pieces of this many samples, about a fifth of a second.
    const CHUNK = 8192;

    class VocalSynthProcessor extends AudioWorkletProcessor {
        constructor(options) {
            super();
            // The page fetches the wasm bytes and passes them in, because worklets cannot fetch.
            const module = new WebAssembly.Module(options.processorOptions.wasm);
            this.wasm = new WebAssembly.Instance(module, {}).exports;
            this.synth = this.wasm.vb_new(sampleRate);
            this.chunk = null;
            this.filled = 0;
            this.port.onmessage = ev => this.receive(ev.data);
        }
        receive(msg) {
            switch (msg.type) {
                case 'set':
                    this.wasm.vb_set(this.synth, msg.id, msg.value);
                    break;
                case 'record':
                    this.chunk = new Float32Array(CHUNK);
                    this.filled = 0;
                    break;
                case 'stop-record':
                    if (this.chunk) {
                        const rest = this.chunk.slice(0, this.filled);
                        this.port.postMessage({ type: 'audio', samples: rest, last: true }, [rest.buffer]);
                    }
                    this.chunk = null;
                    break;
            }
        }
        keep(samples) {
            let from = 0;
            while (from < samples.length) {
                const count = Math.min(samples.length - from, CHUNK - this.filled);
                this.chunk.set(samples.subarray(from, from + count), this.filled);
                this.filled += count;
                from += count;
                if (this.filled === CHUNK) {
                    const full = this.chunk;
                    this.port.postMessage({ type: 'audio', samples: full }, [full.buffer]);
                    this.chunk = new Float32Array(CHUNK);
                    this.filled = 0;
                }
            }
        }
        process(_inputs, outputs) {
            const output = outputs[0];
            const frames = output[0].length;
            const pointer = this.wasm.vb_render(this.synth, frames);
            // Rendering can grow the wasm memory, so view it fresh each time.
            const samples = new Float32Array(this.wasm.memory.buffer, pointer, frames);
            for (const channel of output) {
                channel.set(samples);
            }
            if (this.chunk) {
                this.keep(samples);
            }
            return true;
        }
    }

    registerProcessor('vocal-synth', VocalSynthProcessor);
}

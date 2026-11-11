/*!
 * Vocal Bin Voice Controls
 * The voice presets (Man, Woman, Child) and sliders that shape the speaking voice of Vocal Bin's speech synthesizer.
 * Used by the Phoneme Lab and Text to Speech. `settings` holds [parameter number, value] pairs for the render worker,
 * and an `input` event says when a value changed.
 *
 * <app-voice-controls></app-voice-controls>
 *
 * Authored by Erik Gassler - Stoic Dreams
 * Copyright © 2026 Stoic Dreams - https://www.stoicdreams.com
 * Licensed under the MIT license - https://github.com/StoicDreams/MyFiCDN/blob/main/LICENSE
 */
"use strict"
{
    // Parameter numbers, the same as vocal_synth::param in synth/src/voice.rs.
    const PARAM = { pitch: 0, tract: 1, rd: 2, breath: 3, roughness: 4, volume: 7, rate: 10, intonation: 11 };
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

    const presetButtons = Object.keys(PRESETS).map(name => `<button type="button" data-preset="${name}">${name}</button>`).join('');
    // Native range inputs, not webui-input-range, whose events don't reach a parent. The output is aria-hidden,
    // because outputs are live regions and aria-valuetext already gives the value.
    const sliders = SLIDERS.map(s =>
        `<label class="slider"><span class="name">${s.label}</span><output aria-hidden="true"></output><input type="range" data-key="${s.key}" aria-label="${s.label}" min="${s.min}" max="${s.max}" step="${s.step}" value="${s.value}"></label>`
    ).join('');

    webui.define("app-voice-controls", {
        constructor() {
            const t = this;
            t._values = {};
            // t.template is emptied once it moves into the shadow root, so find everything needed later now.
            t._inputs = [...t.template.querySelectorAll('input[type=range]')];
            t._inputs.forEach(input => {
                t.showValue(input);
                input.addEventListener('input', ev => {
                    // The slider's own event would also reach the page, so send one event for both.
                    ev.stopPropagation();
                    t.showValue(input);
                    t.changed();
                });
            });
            t.template.querySelectorAll('[data-preset]').forEach(button => {
                button.addEventListener('click', () => t.applyPreset(button.dataset.preset));
            });
        },
        props: {
            /** [parameter number, value] pairs, as the render worker takes them. */
            settings: {
                get() { return Object.keys(PARAM).map(key => [PARAM[key], this._values[key]]); },
            },
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
            if (!preset) return;
            t._inputs.forEach(input => {
                if (input.dataset.key in preset) {
                    input.value = preset[input.dataset.key];
                    t.showValue(input);
                }
            });
            t.changed();
        },
        changed() {
            this.dispatchEvent(new Event('input', { bubbles: true }));
        },
        shadowTemplate: `
<fieldset class="presets"><legend>Voice</legend>${presetButtons}</fieldset>
<fieldset class="sliders"><legend>Shape the voice</legend>${sliders}</fieldset>
<style type="text/css">
:host {
display:grid;
gap:var(--padding, 1em);
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
input:focus-visible {
outline:2px solid var(--color-focus, currentColor);
outline-offset:2px;
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

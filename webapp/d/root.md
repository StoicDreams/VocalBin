<webui-data data-page-title="{APP_NAME}" data-page-subtitle="Text to speech, built from scratch"></webui-data>

Type or paste English and our speech synthesizer reads it aloud, highlighting each word as it goes. It builds every sound from scratch, with no recordings and no AI. Shape the voice with the sliders, and save what you hear as a WAV file.

<app-text-to-speech></app-text-to-speech>

## See how it works

<webui-cards card-width="300">
    <webui-card name="Vowel Lab" avatar="microphone" theme="primary" link="/vowel-lab">
        Shape a single vowel from scratch. Move the pitch, the voice quality, and the length of the vocal tract, and hear the change as you drag.
    </webui-card>
    <webui-card name="Phoneme Lab" avatar="music" theme="primary" link="/phoneme-lab">
        Write sounds one at a time in ARPAbet or IPA, and hear exactly what you wrote. Loop it to tune a word by ear.
    </webui-card>
    <webui-card name="How speech is made" avatar="book" theme="primary" link="/guides/how-speech-is-made">
        A guide to the voice, from the vocal folds to intonation, with the synthesizer running on the page so you can hear each idea.
    </webui-card>
</webui-cards>

The pronouncing dictionary is the [CMU Pronouncing Dictionary](https://github.com/cmusphinx/cmudict), copyright Carnegie Mellon University, used under its BSD license.

Everything runs in your browser. Recordings are saved straight to your device as WAV files.

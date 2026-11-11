<webui-data data-page-title="Text to Speech" data-page-subtitle="English, read aloud"></webui-data>

Type or paste English and our speech synthesizer reads it aloud, highlighting each word as it goes. It builds every sound from scratch, with no recordings and no AI. Shape the voice with the sliders, and save what you hear as a WAV file.

<app-text-to-speech></app-text-to-speech>

## How it works

- **Words to sounds:** each word is looked up in the CMU Pronouncing Dictionary, which lists how about 126,000 English words are said. A word it doesn't have is built from one it does, like "unfriended" from "friend", or else from spelling rules in the style of the Naval Research Laboratory's 1976 rules.
- **Same spelling, different words:** "read", "live", "record", and others are said by how they are used, judged from the words around them.
- **Numbers and symbols:** numbers, dates, times, money, percentages, units, and common abbreviations like "Dr." are read as words. Short acronyms like "FBI" are spelled out.
- **Intonation:** small words like "the" and "of" are unstressed, the important words are raised, and the voice drifts down through a sentence. Statements fall at the end, yes-or-no questions rise, and a comma lifts the voice a little to show there is more.
- **The voice:** the same synthesizer as the [Phoneme Lab](/phoneme-lab), which speaks the sounds one at a time, and the [Vowel Lab](/vowel-lab), which shows how a single vowel is made.

The pronouncing dictionary is the [CMU Pronouncing Dictionary](https://github.com/cmusphinx/cmudict), copyright Carnegie Mellon University, used under its BSD license.

Everything runs in your browser. Recordings are saved straight to your device as WAV files.

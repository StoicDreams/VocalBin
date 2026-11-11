<webui-data data-page-title="Phoneme Lab" data-page-subtitle="Speech from sounds"></webui-data>

The second stage of our speech synthesizer speaks whole words, one sound at a time. Write them in ARPAbet, the phoneme alphabet of the CMU Pronouncing Dictionary, or in IPA, the International Phonetic Alphabet used in dictionaries and linguistics. Our synthesizer builds every sound from scratch, with no recordings and no AI. Pick an example to start, or open the phoneme chart.

<app-phoneme-lab></app-phoneme-lab>

## How it works

- **Vowels and their neighbors:** the same vocal folds and vocal tract as the [Vowel Lab](/vowel-lab). Between sounds, the formants glide the way the tongue and lips move, which is how you hear the difference between "ba", "da", and "ga".
- **Hiss and bursts:** sounds like S, SH, and F are noise from a narrow gap, shaped by their own filters. P, T, and K close the mouth, release a short burst, and breathe before the voice starts.
- **The nose:** M, N, and NG add the nasal cavity's own resonance and the notch it cuts into the sound.
- **Timing and pitch:** stressed vowels are longer and higher, the voice drifts down through a sentence, and a question rises at the end.

Everything runs in your browser. Recordings are saved straight to your device as WAV files.

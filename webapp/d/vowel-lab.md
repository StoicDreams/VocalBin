<webui-data data-page-title="Vowel Lab" data-page-subtitle="Shape a voice from scratch"></webui-data>

We are building a speech synthesizer that makes every sound from scratch. A model of the vocal folds feeds a model of the vocal tract, with no recordings and no AI. This lab is its first stage: pick a vowel, shape the voice, and record what you make.

<app-vowel-lab></app-vowel-lab>

## How it works

- **The vocal folds:** each pulse of air follows the Liljencrants–Fant model of the glottis. Voice quality changes the pulse's shape, from a pressed voice to a lax one, and roughness adds the small differences between real pulses.
- **The vocal tract:** five resonances, called formants, shape the pulses into a vowel. A shorter vocal tract raises them all, which is most of the difference between a man's, a woman's, and a child's voice.
- **Moving between vowels:** the formants glide to each new vowel the way the tongue and lips move, speeding up and then settling, instead of jumping.

Everything runs in your browser. Recordings are saved straight to your device as WAV files.

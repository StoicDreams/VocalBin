<webui-data data-page-title="How Speech Is Made" data-page-subtitle="Source, filter, and a synthesizer built from them"></webui-data>
<webui-data data-page-description="How the voice makes speech: the vocal folds as a source, the vocal tract as a filter, formants and vowels, consonants, and intonation, with a speech synthesizer running on the page to hear each idea."></webui-data>

*By Erik Gassler · November 11, 2026*

Every sound you speak starts as air from your lungs, and two things turn that air into speech. A **source** makes sound, and a **filter** shapes it. Phoneticians have described speech this way since Gunnar Fant's *Acoustic Theory of Speech Production* in 1960, and it is exactly how Vocal Bin's synthesizer is built. This guide walks through each part, with the synthesizer running on the page so you can hear every idea for yourself.

## The source: the vocal folds

In your throat, the larynx holds two folds of tissue, the vocal folds. When you speak a vowel, air pressure pushes them apart, and they snap shut again, over and over. Each time they close, the air flow stops sharply, and that sudden stop is a pulse of sound. A run of pulses is a buzz, rich in overtones.

- **Pitch** is how often the folds close: around 100 to 150 times a second in a typical man's voice, and around 200 in a woman's. Children's voices are higher still.
- **Voice quality** is the shape of each pulse. Folds pressed tightly together close fast and sound tense and bright. Loosely held folds close gently and sound soft and lax. Our synthesizer shapes each pulse with the Liljencrants–Fant model, which sets that whole range with one number.
- **Breathiness** is air that leaks through folds that never quite close, heard as a hiss under the voice.
- **Roughness** comes from real pulses never being exactly alike. Small differences in their timing and strength (jitter and shimmer) are part of what makes a voice sound human rather than electronic.

Some sounds use no buzz at all. In "s", "f", and "h", the source is the hiss of air rushing through a narrow gap.

Try it here. Press **Play**, then move **Pitch**, and slide **Voice quality** from pressed to lax. The vowel stays the same while the voice changes, because these sliders only change the source.

<app-vowel-lab></app-vowel-lab>

## The filter: the vocal tract

The buzz from the folds then passes through the throat, mouth, and lips: the vocal tract, a tube about 17.5 centimeters long in an adult man. Like air in a bottle, the tube resonates at certain frequencies, and the parts of the buzz near those frequencies come out stronger. These resonances are called **formants**, and they are what make one vowel sound different from another.

The first two formants do most of the work:

- **F1** rises as the mouth opens. The open vowel in "hod" has a high F1, and the close vowel in "heed" a low one.
- **F2** rises as the tongue moves forward. "Heed" has the tongue far forward and a high F2, and "who'd" has it far back and a low F2.

Plot the vowels by those two numbers, with F2 running right to left and F1 running down, and you get a map of the mouth. Front vowels sit on the left, back vowels on the right, and open vowels at the bottom. The averages below come from a 1995 study of American English speakers by Hillenbrand and colleagues, and they are the targets our synthesizer aims for.

<app-vowel-chart></app-vowel-chart>

Now switch to **Woman** or **Child**. A shorter vocal tract raises every formant by the same proportion, so the whole map slides up and to the left while keeping its shape. That is most of the difference between a man's, a woman's, and a child's voice saying the same vowel, and listeners hear the vowels as the same partly because of how they sit relative to each other. In the Vowel Lab above, the **Vocal tract length** slider does exactly this.

Our synthesizer uses five formants in a chain, the layout Dennis Klatt published for his formant synthesizer in 1980. The vowel sets the first three, and the upper two stay nearly fixed, as they do in real voices.

## Consonants: closing and narrowing the tract

Consonants come from blocking or narrowing the vocal tract somewhere along its length.

- **Stops** ("p", "t", "k", "b", "d", "g") close the tract completely. Pressure builds up behind the closure, and releasing it makes a short burst. Where the closure is (the lips, the ridge behind the teeth, or the soft palate) changes the burst's sound, and it also bends the formants of the vowel next to it as the tongue moves away. At the start of a stressed syllable, English "p", "t", and "k" also have a puff of breath before the voice starts, which is much of what tells "pin" from "bin".
- **Fricatives** ("s", "sh", "f", "th") narrow the tract until the air hisses. "Sh" is the loudest of them, and "f" and "th" are so quiet they are easy to mishear on a phone call.
- **Nasals** ("m", "n", "ng") close the mouth and lower the soft palate, so the sound goes out through the nose. The closed mouth becomes a side branch that cancels some frequencies, which gives nasals their muffled quality.
- **Approximants** ("l", "r", "w", "y") narrow the tract only a little. They behave like quick vowels, gliding into the sound after them.

You can hear each of these on its own in the [Phoneme Lab](/phoneme-lab), which speaks sounds you write one at a time, in ARPAbet or the International Phonetic Alphabet.

## Prosody: the music of a sentence

The same words can mean different things depending on how they are said. Pitch, timing, and loudness together are called **prosody**.

- **Stress:** stressed syllables are longer and louder, and usually carry a change in pitch. Small words like "the", "of", and "to" are said quickly with a weak vowel, so "a cup of tea" sounds like "a cuppa tea".
- **Intonation:** the voice drifts down through a sentence. A statement falls at its end, and a yes-or-no question rises. "It's raining." and "It's raining?" differ only in this.
- **Pauses:** commas, sentence ends, and breaths in long sentences break speech into phrases that are easier to follow.

## From text to speech

To read written text aloud, a synthesizer has to work out what to say before it can say it:

1. **Normalize the text.** "Dr. Smith paid $5.50 on 11/11" becomes "doctor smith paid five dollars and fifty cents on november eleventh".
2. **Find each word's sounds.** Our reader looks words up in the CMU Pronouncing Dictionary, which lists over 126,000 English words. For words it lacks, it builds them from parts it knows ("unfriended" from "friend"), and failing that it uses letter-to-sound rules in the style of a 1976 set from the US Naval Research Laboratory.
3. **Choose between words spelled the same.** "I read it yesterday" and "I will read it" need different vowels, judged from the words around them.
4. **Plan the prosody,** then hand the sounds, their lengths, and their pitches to the synthesizer.

[Text to Speech](/text-to-speech) does all of this in your browser. Open its Pronunciation section to see the sounds it chose, written in IPA.

## Why build a voice this way

Most text-to-speech today uses neural networks trained on many hours of recorded speech. Formant synthesis is the older approach. Stephen Hawking's famous voice came from a formant synthesizer based on Dennis Klatt's work. A formant synthesizer needs no recordings, it is small enough to run anywhere, and every part of the voice is a number you can change and understand.

Formant synthesis has a reputation for sounding robotic, and early systems earned it. We think a voice built from these parts can sound far more natural than that reputation suggests, and Vocal Bin is our attempt to find out.

## Further reading

- Gunnar Fant (1960). *Acoustic Theory of Speech Production.* Mouton.
- Dennis H. Klatt (1980). "Software for a cascade/parallel formant synthesizer." *Journal of the Acoustical Society of America* 67(3).
- Gunnar Fant, Johan Liljencrants, and Qi-guang Lin (1985). "A four-parameter model of glottal flow." *STL-QPSR* 26(4), KTH Stockholm.
- James Hillenbrand, Laura A. Getty, Michael J. Clark, and Kimberlee Wheeler (1995). "Acoustic characteristics of American English vowels." *Journal of the Acoustical Society of America* 97(5).
- Honey S. Elovitz, Rodney Johnson, Astrid McHugh, and John E. Shore (1976). *Automatic Translation of English Text to Phonetics by Means of Letter-to-Sound Rules.* NRL Report 7948, Naval Research Laboratory.
- The [CMU Pronouncing Dictionary](https://github.com/cmusphinx/cmudict), Carnegie Mellon University.

Everything runs in your browser. Recordings are saved straight to your device as WAV files.

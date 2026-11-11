//! Vocal Bin's speech synthesizer. Classic synthesis only: a model of the vocal folds feeds a model of the vocal
//! tract, with no recordings and no AI models.
//!
//! - [`Voice`] makes the sound: an LF glottal source and aspiration through a cascade of formants, and frication
//!   noise through parallel resonators.
//! - Stage 1, [`Synth`]: one vowel at a time, gliding between them. The vowel lab runs it in an `AudioWorklet`.
//! - Stage 2, [`Speaker`]: phonemes in IPA or ARPAbet, rendered ahead of playback in a Web Worker for the phoneme lab.
//! - Stage 3: ordinary text, read by the language with a pronouncing [`Dictionary`] loaded from a file, with
//!   intonation. The text-to-speech page uses the same worker.
//! - Sounds are named in IPA. Each [`Language`] brings its own table of them, so adding a language means adding a
//!   table like [`ENGLISH`], not changing the speaker.
//!
//! The pages load this crate as raw WebAssembly through the `vb_*` functions below, without wasm-bindgen.

mod dictionary;
mod english;
mod filter;
mod glottal;
mod language;
mod phonemes;
mod speaker;
mod synth;
mod voice;
mod vowels;

pub use filter::{Glide, Resonator};
pub use glottal::LfPulse;
pub use dictionary::{Dictionary, Sound};
pub use english::{ARPABET, Arpabet, ENGLISH, parse_arpabet};
pub use language::{Language, Notation, Reading, TextReader};
pub use phonemes::{Kind, ParseError, Phoneme, Place, Token};
pub use speaker::{Item, Speaker};
pub use synth::Synth;
pub use voice::{Controls, Voice, param};
pub use vowels::{VOWELS, Vowel};

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::Synth;

    /// Creates a synthesizer. Free it with `vb_free`.
    #[unsafe(no_mangle)]
    pub extern "C" fn vb_new(sample_rate: f32) -> *mut Synth {
        Box::into_raw(Box::new(Synth::new(sample_rate)))
    }

    /// # Safety
    /// `synth` must come from `vb_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_free(synth: *mut Synth) {
        if !synth.is_null() {
            drop(unsafe { Box::from_raw(synth) });
        }
    }

    /// Sets a parameter. The numbers are in `vocal_synth::param`.
    ///
    /// # Safety
    /// `synth` must come from `vb_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_set(synth: *mut Synth, id: u32, value: f32) {
        if let Some(synth) = unsafe { synth.as_mut() } {
            synth.set(id, value);
        }
    }

    /// Renders `frames` samples and returns where they are in memory. They stay valid until the next call.
    /// Make a new view of the memory each time, since rendering can grow it.
    ///
    /// # Safety
    /// `synth` must come from `vb_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_render(synth: *mut Synth, frames: u32) -> *const f32 {
        match unsafe { synth.as_mut() } {
            Some(synth) => synth.render_block(frames as usize).as_ptr(),
            None => std::ptr::null(),
        }
    }

    use crate::Speaker;

    /// Reserves `len` bytes for the page to write text into. Free it with `vb_dealloc`.
    #[unsafe(no_mangle)]
    pub extern "C" fn vb_alloc(len: u32) -> *mut u8 {
        let mut bytes = Vec::<u8>::with_capacity(len as usize);
        let pointer = bytes.as_mut_ptr();
        std::mem::forget(bytes);
        pointer
    }

    /// # Safety
    /// `pointer` and `len` must come from one call to `vb_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_dealloc(pointer: *mut u8, len: u32) {
        drop(unsafe { Vec::from_raw_parts(pointer, 0, len as usize) });
    }

    /// Creates a speaker. Free it with `vb_speaker_free`.
    #[unsafe(no_mangle)]
    pub extern "C" fn vb_speaker_new(sample_rate: f32) -> *mut Speaker {
        Box::into_raw(Box::new(Speaker::new(sample_rate)))
    }

    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_free(speaker: *mut Speaker) {
        if !speaker.is_null() {
            drop(unsafe { Box::from_raw(speaker) });
        }
    }

    /// Sets a voice setting, or the rate or intonation for the next text. The numbers are in `vocal_synth::param`.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_set(speaker: *mut Speaker, id: u32, value: f32) {
        if let Some(speaker) = unsafe { speaker.as_mut() } {
            speaker.set(id, value);
        }
    }

    /// Prepares the UTF-8 text at `text` to be rendered, written in `notation` (0 ARPAbet, 1 IPA, 2 ordinary text
    /// without a dictionary). Returns -1 when it reads, or else the byte offset of the first thing it can't read,
    /// whose length `vb_speaker_error_len` then gives.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new`, and `text` must point to `len` bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_speak(speaker: *mut Speaker, text: *const u8, len: u32, notation: u32) -> i32 {
        let Some(speaker) = (unsafe { speaker.as_mut() }) else { return 0 };
        let Some(notation) = crate::Notation::from_number(notation) else { return 0 };
        let bytes = unsafe { std::slice::from_raw_parts(text, len as usize) };
        let Ok(text) = std::str::from_utf8(bytes) else { return 0 };
        report(speaker.speak(text, notation))
    }

    fn report(result: Result<(), crate::ParseError>) -> i32 {
        match result {
            Ok(()) => {
                ERROR_LEN.with(|e| e.set(0));
                -1
            }
            Err(error) => {
                ERROR_LEN.with(|e| e.set(error.len as u32));
                error.offset as i32
            }
        }
    }

    use crate::Dictionary;

    /// Reads a pronouncing dictionary file for the speakers' language (English) from the `len` bytes at `bytes`.
    /// Returns null if the file can't be read. Free it with `vb_dictionary_free`.
    ///
    /// # Safety
    /// `bytes` must point to `len` bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_dictionary_load(bytes: *const u8, len: u32) -> *mut Dictionary {
        let bytes = unsafe { std::slice::from_raw_parts(bytes, len as usize) };
        match Dictionary::parse(bytes, &crate::ENGLISH) {
            Ok(dictionary) => Box::into_raw(Box::new(dictionary)),
            Err(_) => std::ptr::null_mut(),
        }
    }

    /// # Safety
    /// `dictionary` must come from `vb_dictionary_load` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_dictionary_free(dictionary: *mut Dictionary) {
        if !dictionary.is_null() {
            drop(unsafe { Box::from_raw(dictionary) });
        }
    }

    /// Prepares ordinary UTF-8 text at `text` to be rendered, read with `dictionary` (which may be null). Returns the
    /// same as `vb_speaker_speak`. Afterwards `vb_speaker_transcript` has it in IPA.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new`, `dictionary` from `vb_dictionary_load` or be null, and `text` must
    /// point to `len` bytes.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_speak_text(speaker: *mut Speaker, dictionary: *const Dictionary, text: *const u8, len: u32) -> i32 {
        let Some(speaker) = (unsafe { speaker.as_mut() }) else { return 0 };
        let bytes = unsafe { std::slice::from_raw_parts(text, len as usize) };
        let Ok(text) = std::str::from_utf8(bytes) else { return 0 };
        report(speaker.speak_text(text, unsafe { dictionary.as_ref() }))
    }

    /// Where the transcript of the last ordinary text is, as UTF-8. `vb_speaker_transcript_len` gives its length.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_transcript(speaker: *const Speaker) -> *const u8 {
        unsafe { speaker.as_ref() }.map_or(std::ptr::null(), |s| s.transcript().as_ptr())
    }

    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_transcript_len(speaker: *const Speaker) -> u32 {
        unsafe { speaker.as_ref() }.map_or(0, |s| s.transcript().len() as u32)
    }

    thread_local! {
        static ERROR_LEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    }

    /// The length in bytes of the text `vb_speaker_speak` last couldn't read.
    #[unsafe(no_mangle)]
    pub extern "C" fn vb_speaker_error_len() -> u32 {
        ERROR_LEN.with(|e| e.get())
    }

    /// The length of the prepared text in samples.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_length(speaker: *const Speaker) -> u32 {
        unsafe { speaker.as_ref() }.map_or(0, |s| s.length())
    }

    /// Renders `frames` samples and returns where they are in memory. They stay valid until the next call.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_render(speaker: *mut Speaker, frames: u32) -> *const f32 {
        match unsafe { speaker.as_mut() } {
            Some(speaker) => speaker.render_block(frames as usize).as_ptr(),
            None => std::ptr::null(),
        }
    }

    /// How many phonemes and pauses the prepared text has.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_items(speaker: *const Speaker) -> u32 {
        unsafe { speaker.as_ref() }.map_or(0, |s| s.items().len() as u32)
    }

    /// One field of item `index`: 0 its start and 1 its end in samples, 2 its byte offset and 3 its length in the text.
    ///
    /// # Safety
    /// `speaker` must come from `vb_speaker_new` and not have been freed.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn vb_speaker_item(speaker: *const Speaker, index: u32, field: u32) -> u32 {
        let Some(item) = unsafe { speaker.as_ref() }.and_then(|s| s.items().get(index as usize)) else { return 0 };
        match field {
            0 => item.start,
            1 => item.end,
            2 => item.offset,
            _ => item.len,
        }
    }
}

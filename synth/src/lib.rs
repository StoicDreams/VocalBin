//! Vocal Bin's speech synthesizer. Classic synthesis only: a model of the vocal folds feeds a model of the vocal
//! tract, with no recordings and no AI models.
//!
//! Stage 1 is vowels: [`Synth`] voices one of [`VOWELS`] and glides between them. The page loads this crate as raw
//! WebAssembly in an `AudioWorklet`, through the `vb_*` functions below, without wasm-bindgen.

mod filter;
mod glottal;
mod synth;
mod vowels;

pub use filter::{Glide, Resonator};
pub use glottal::LfPulse;
pub use synth::{Synth, param};
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
}

#[macro_use]
extern crate failure;

pub type SynthResult<T> = Result<T, failure::Error>;

pub mod context;
pub mod synth;

pub use context::Context;
pub use synth::Synth;
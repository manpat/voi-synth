#[macro_use]
extern crate failure;

pub type SynthResult<T> = Result<T, failure::Error>;

pub mod context;
pub mod synth;
pub mod buffer;

pub use context::Context;
pub use synth::Synth;
pub use buffer::Buffer;
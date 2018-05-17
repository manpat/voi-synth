#[macro_use]
extern crate failure;

pub type SynthResult<T> = Result<T, failure::Error>;

pub mod context;
pub mod synth;
pub mod buffer;

pub use context::Context;
pub use synth::{Synth, Node, Input, Phase};
pub use buffer::Buffer;

fn lerp(from: f32, to: f32, amt: f32) -> f32 {
	from + (to-from) * amt
}
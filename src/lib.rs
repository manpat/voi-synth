#![feature(nll)]
#![feature(integer_atomics)]

// #[macro_use]
pub extern crate failure;

pub type SynthResult<T> = Result<T, failure::Error>;

pub mod context;
pub mod synth;
pub mod node;
pub mod buffer;
mod parameter;
mod envelope;
mod gate;

pub use context::Context;
pub use synth::{Synth, SynthID};
pub use node::{NodeID, NodeContainer};
pub use parameter::ParameterID;
pub use buffer::Buffer;

fn lerp(from: f32, to: f32, amt: f32) -> f32 {
	from + (to-from) * amt
}

/*

TODO
====

Allow creation of triggerable, and fillable audiobuffers
	Load from file or prerender a synth to a buffer for later playback

Buffer based effects
	Delay lines - basically looping audio buffers with some extra behaviour 
	Granular synthesis

Add module for interacting with midi

Add module for sequencing/timing
	notion of triggers

	trigger conditions
		trigger likelihood - trigger n% of times
		signal divisor? - trigger every n times

Add scripting module

*/
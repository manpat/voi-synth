use SynthResult;

pub mod flags {
	// const 
}

pub struct Synth {
	flags: u32,
}

impl Synth {
	pub fn new() -> Self {
		Synth {
			flags: 0,
		}
	}
}
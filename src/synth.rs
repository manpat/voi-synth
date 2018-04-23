use buffer::Buffer;

use std::f64::consts::PI;

pub mod flags {
	// const 
}

#[derive(Clone, Debug)]
pub struct Synth {
	flags: u32,
	phase: f64,
	freq: f64,
}

impl Synth {
	pub fn new() -> Self {
		Synth {
			flags: 0,
			phase: 0.0,
			freq: 110.0,
		}
	}

	pub fn with_freq(freq: f64) -> Self {
		Synth {
			flags: 0,
			phase: 0.0,
			freq,
		}
	}

	pub fn evaluate_into_buffer(&mut self, buffer: &mut Buffer) {
		for vs in buffer.data.chunks_mut(2) {
			let value = (self.phase * PI * 2.0).sin() * 0.04;

			vs[0] += value as f32;
			vs[1] += value as f32;

			self.phase += self.freq / 22050.0;
			self.phase = self.phase % 1.0;
		}
	}
}

use crate::node::{Input, InputContext};
use crate::gate::{Gate, GateState};

#[derive(Copy, Clone, Debug)]
enum State {
	Silence, Attack, Decay, Sustain, Release,
}

#[derive(Clone, Debug)]
pub struct ADSR {
	state: State,
	position: f32,

	gate: Gate,

	atk_inc: f32,
	dec_inc: f32,
	sus_lvl: f32,
	rel_inc: f32,
}

impl ADSR {
	pub fn new<G: Into<Input>>(atk: f32, dec: f32, sus: f32, rel: f32, gate: G) -> ADSR {
		let sus_lvl = sus.max(0.0).min(1.0);

		ADSR {
			state: State::Silence,
			position: 0.0,

			gate: Gate::new(gate.into()),

			// NOTE: this model allows doesn't allow decay to be cancelled on gate falling edge
			// this may or may not be desirable but needs thought
			atk_inc: 1.0 / atk.max(0.00001),
			dec_inc: (1.0 - sus_lvl) / dec.max(0.00001),
			sus_lvl,
			rel_inc: sus_lvl / rel.max(0.00001),
		}
	}

	fn update(&mut self, gate: GateState, dt: f32) {
		use self::State::*;

		self.state = match self.state {
			Silence => if gate.is_rising_edge() {
				self.position = 0.0;
				Attack
			} else {
				Silence
			}

			Attack => {
				self.position += self.atk_inc * dt;

				if self.position >= 1.0 {
					self.position = 1.0;
					Decay
				} else {
					Attack
				}
			}

			Decay => {
				self.position -= self.dec_inc * dt;

				if gate.is_rising_edge() {
					Attack
				} else if self.position <= self.sus_lvl {
					self.position = self.sus_lvl;
					Sustain
				} else {
					Decay
				}
			}

			Sustain => if gate.is_lowish() {
				Release
			} else if gate.is_rising_edge() {
				Attack
			} else {
				Sustain
			}

			Release => {
				self.position -= self.rel_inc * dt;

				if gate.is_rising_edge() {
					Attack
				} else if self.position <= 0.0 {
					self.position = 0.0;
					Silence
				} else {
					Release
				}
			}
		}
	}

	pub fn advance(&mut self, input_ctx: InputContext) -> f32 {
		let sample = self.position;
		let gate = self.gate.update(input_ctx);
		self.update(gate, input_ctx.eval_ctx.sample_dt);
		sample
	}
}


#[derive(Clone, Debug)]
pub struct AR {
	state: State,
	position: f32,

	gate: Gate,

	// in u/s
	atk_inc: f32,
	rel_inc: f32,
}

impl AR {
	pub fn new<G: Into<Input>>(atk: f32, rel: f32, gate: G) -> AR {
		AR {
			state: State::Silence,
			position: 0.0,

			gate: Gate::new(gate.into()),

			atk_inc: 1.0 / atk.max(0.00001),
			rel_inc: 1.0 / rel.max(0.00001),
		}
	}

	fn update(&mut self, gate: GateState, inc: f32) {
		use self::State::*;

		self.state = match self.state {
			Silence => if gate.is_rising_edge() {
				self.position = 0.0;
				Attack
			} else {
				Silence
			}

			Attack => {
				self.position += self.atk_inc * inc;
				if self.position >= 1.0 {
					self.position = 1.0;
					Release
				} else {
					Attack
				}
			}

			Release => {
				self.position -= self.rel_inc * inc;
				if self.position <= 0.0 {
					self.position = 0.0;

					if gate.is_rising_edge() { Attack }
					else { Silence }

				} else {
					if gate.is_rising_edge() { Attack }
					else { Release }
				}
			}

			_ => Silence
		}
	}

	pub fn advance(&mut self, input_ctx: InputContext) -> f32 {
		let sample = self.position;
		let gate = self.gate.update(input_ctx);
		self.update(gate, input_ctx.eval_ctx.sample_dt);
		sample
	}
}
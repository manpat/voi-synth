use node::{Input, InputContext};

#[derive(Copy, Clone, Debug)]
enum State {
	Silence, Attack, Decay, Sustain, Release,
}

#[derive(Clone, Debug)]
pub struct ADSR {
	state: State,
	position: f32,

	gate: Input,

	atk_inc: f32,
	dec_inc: f32,
	sus_lvl: f32,
	rel_inc: f32,
}

impl ADSR {
	pub fn new(gate: Input) -> ADSR {
		ADSR {
			state: State::Silence,
			position: 0.0,

			gate,

			atk_inc: 0.0,
			dec_inc: 0.0,
			sus_lvl: 0.0,
			rel_inc: 0.0,
		}
	}

	fn update(&mut self, gate: f32) {
		use self::State::*;

		match self.state {
			Silence => if gate > 0.0 { self.state = Attack; self.position = 0.0; }
			Attack => {}
			Decay => {}
			Sustain => {}
			Release => {}
		}
	}

	pub fn advance(&mut self, input_ctx: InputContext) -> f32 {
		let sample = self.position;
		let gate = self.gate.evaluate(input_ctx);
		self.update(gate);
		sample
	}
}


#[derive(Clone, Debug)]
pub struct AR {
	state: State,
	position: f32,

	gate: Input,

	// in u/s
	atk_inc: f32,
	rel_inc: f32,
}

impl AR {
	pub fn new(atk: f32, rel: f32, gate: Input) -> AR {
		AR {
			state: State::Silence,
			position: 0.0,

			gate,

			atk_inc: 1.0 / atk.max(0.00001),
			rel_inc: 1.0 / rel.max(0.00001),
		}
	}

	fn update(&mut self, gate: f32, inc: f32) {
		use self::State::*;

		self.state = match self.state {
			Silence => if gate > 0.5 { self.position = 0.0; Attack } else { Silence }

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
					if gate < 0.5 { Silence } else { Release }
				} else {
					Release
				}
			}

			_ => Silence
		}
	}

	pub fn advance(&mut self, input_ctx: InputContext) -> f32 {
		let sample = self.position;
		let gate = self.gate.evaluate(input_ctx);
		self.update(gate, input_ctx.eval_ctx.sample_dt);
		sample
	}
}
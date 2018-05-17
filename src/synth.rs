use buffer::Buffer;
use context::EvaluationContext;

use lerp;

use std::f32::consts::PI;

pub mod flags {
	// const 
}

#[derive(Clone, Debug)]
pub struct Synth {
	pub id: u32,
	pub flags: u32,

	gain: f32,

	instructions: Vec<Node>,
	// interpolators: Vec<Interpolator>,
}

impl Synth {
	pub fn new() -> Self {
		Synth {
			id: 0,
			flags: 0,

			gain: 1.0,

			instructions: Vec::new(),
		}
	}

	pub fn push_node(&mut self, inst: Node) -> NodeID {
		let id = self.instructions.len() as _;
		self.instructions.push(inst);
		NodeID(id)
	}

	pub fn set_gain(&mut self, gain: f32) { self.gain = gain }

	fn evaluate_sample(&mut self, eval_ctx: &mut EvaluationContext) -> f32 {
		assert!(eval_ctx.sample_arena.len() >= self.instructions.len());

		for (idx, inst) in self.instructions.iter_mut().enumerate() {
			let sample = match *inst {
				Node::Sine{ref mut phase} => phase.advance(eval_ctx).sin(),
				Node::Saw{ref mut phase} => phase.advance(eval_ctx) * 2.0 - 1.0,
				Node::Square{ref mut phase, ..} => 1.0 - (phase.advance(eval_ctx) + 0.5).floor() * 2.0,
				Node::Triangle{ref mut phase} => {
					let ph = phase.advance(eval_ctx);
					if ph <= 0.5 {
						(ph - 0.25)*4.0
					} else {
						(0.75 - ph)*4.0
					}
				}


				Node::LowPass{input, freq, ref mut prev_result} => {
					let sample = input.evaluate(eval_ctx);
					let cutoff = freq.evaluate(eval_ctx);

					if cutoff > 0.0 {
						let dt = 1.0 / eval_ctx.sample_rate;
						let a = dt / (dt + 1.0 / (2.0 * PI * cutoff));
						*prev_result = lerp(*prev_result, sample, a);
					} else {
						*prev_result = 0.0;
					}
						
					*prev_result
				}

				Node::HighPass{input, freq, ref mut prev_result, ref mut prev_sample} => {
					let sample = input.evaluate(eval_ctx);
					let cutoff = freq.evaluate(eval_ctx);

					let rc = 1.0 / (2.0 * PI * cutoff);
					let a = rc / (rc + 1.0/eval_ctx.sample_rate);

					let result = a * (*prev_result + sample - *prev_sample);
					*prev_sample = sample;
					*prev_result = result;
						
					result
				}


				Node::Add(a, b) => a.evaluate(eval_ctx) + b.evaluate(eval_ctx),
				Node::Subtract(a, b) => a.evaluate(eval_ctx) - b.evaluate(eval_ctx),
				Node::Multiply(a, b) => a.evaluate(eval_ctx) * b.evaluate(eval_ctx),
				Node::Divide(a, b) => a.evaluate(eval_ctx) / b.evaluate(eval_ctx),
				Node::Power(a, b) => a.evaluate(eval_ctx).powf(b.evaluate(eval_ctx)),

				_ => unimplemented!()
			};

			unsafe {
				*eval_ctx.sample_arena.get_unchecked_mut(idx) = sample;
			}
		}

		if self.instructions.len() > 0 {
			unsafe {
				*eval_ctx.sample_arena.get_unchecked(self.instructions.len()-1)
			}
		} else {
			0.0
		}
	}

	pub fn evaluate_into_buffer(&mut self, buffer: &mut Buffer, eval_ctx: &mut EvaluationContext) {
		if eval_ctx.sample_arena.len() < self.instructions.len() {
			eval_ctx.sample_arena.resize(self.instructions.len(), 0.0);
		}

		// TODO: Make sure instructions are evaluated in order. i.e., make sure dependencies are evaluated before terminal nodes

		for vs in buffer.data.chunks_mut(2) {
			let value = self.evaluate_sample(eval_ctx) * self.gain;

			vs[0] += value as f32;
			vs[1] += value as f32;
		}
	}
}



// NOTE: evaluation of Input::Node assumes that dependent nodes are evaluated before terminal nodes
// If this invariant is violated, nodes will get stale samples
#[derive(Debug, Clone, Copy)]
pub enum Input {
	Literal(f32),
	Node(NodeID),
}

impl Input {
	fn evaluate(self, eval_ctx: &EvaluationContext) -> f32 {
		// TODO: Profile, [f32]::get_unchecked
		match self {
			Input::Literal(f) => f,
			Input::Node(NodeID(idx)) => eval_ctx.sample_arena[idx as usize]
		}
	}
}

impl Into<Input> for f32 {
	fn into(self) -> Input { Input::Literal(self) }
}

impl Into<Input> for NodeID {
	fn into(self) -> Input { Input::Node(self) }
}



#[derive(Debug, Clone, Copy)]
pub struct Phase {
	phase: f32,
	period: f32,

	freq: Input,
}

impl Phase {
	pub fn new(freq: Input) -> Phase {
		Phase {
			phase: 0.0,
			period: 1.0,

			freq
		}
	}

	pub fn with_period(freq: Input, period: f32) -> Phase {
		Phase {
			phase: 0.0,
			period,

			freq
		}
	}

	pub fn advance(&mut self, eval_ctx: &EvaluationContext) -> f32 {
		let freq = self.freq.evaluate(eval_ctx);

		self.phase += self.period * freq as f32 / eval_ctx.sample_rate as f32;
		self.phase %= self.period;
		self.phase as f32
	}
}



#[derive(Debug, Copy, Clone)]
pub struct NodeID (u32);

#[derive(Debug, Clone)]
pub enum Node {
	Sine{ phase: Phase },
	Triangle{ phase: Phase },
	Square{ phase: Phase, width: Input },
	Saw{ phase: Phase },
	Noise{ phase: Phase },

	LowPass{ input: Input, freq: Input, prev_result: f32 },
	HighPass{ input: Input, freq: Input, prev_sample: f32, prev_result: f32 },

	Add(Input, Input),
	Subtract(Input, Input),
	Multiply(Input, Input),
	Divide(Input, Input),
	Power(Input, Input),
}

impl Node {
	pub fn new_sine<I: Into<Input>>(freq: I) -> Node {
		Node::Sine{ phase: Phase::with_period(freq.into(), 2.0 * PI) }
	}

	pub fn new_triangle<I: Into<Input>>(freq: I) -> Node {
		Node::Triangle{ phase: Phase::new(freq.into()) }
	}

	pub fn new_saw<I: Into<Input>>(freq: I) -> Node {
		Node::Saw{ phase: Phase::new(freq.into()) }
	}

	pub fn new_square<I: Into<Input>, I2: Into<Input>>(freq: I, width: I2) -> Node {
		Node::Square{ phase: Phase::new(freq.into()), width: width.into() }
	}



	pub fn new_lowpass<I: Into<Input>, I2: Into<Input>>(input: I, freq: I2) -> Node {
		Node::LowPass{ input: input.into(), freq: freq.into(), prev_result: 0.0 }
	}
	pub fn new_highpass<I: Into<Input>, I2: Into<Input>>(input: I, freq: I2) -> Node {
		Node::HighPass{ input: input.into(), freq: freq.into(), prev_result: 0.0, prev_sample: 0.0 }
	}


	pub fn new_add<I: Into<Input>, I2: Into<Input>>(a: I, b: I2) -> Node { Node::Add(a.into(), b.into()) }
	pub fn new_sub<I: Into<Input>, I2: Into<Input>>(a: I, b: I2) -> Node { Node::Subtract(a.into(), b.into()) }
	pub fn new_multiply<I: Into<Input>, I2: Into<Input>>(a: I, b: I2) -> Node { Node::Multiply(a.into(), b.into()) }
	pub fn new_divide<I: Into<Input>, I2: Into<Input>>(a: I, b: I2) -> Node { Node::Divide(a.into(), b.into()) }
	pub fn new_power<I: Into<Input>, I2: Into<Input>>(a: I, b: I2) -> Node { Node::Power(a.into(), b.into()) }
}
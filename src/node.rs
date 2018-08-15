use synth::{Synth, StoreID};
use buffer::{BufferID, BufferSampler};
use context::EvaluationContext;

use envelope as env;

use std::f32::consts::PI;

// NOTE: evaluation of Input::Node assumes that dependent nodes are evaluated before terminal nodes
// If this invariant is violated, nodes will get stale samples
#[derive(Debug, Clone, Copy)]
pub enum Input {
	Literal(f32),
	Node(NodeID),
	Store(StoreID)
}

#[derive(Clone, Copy)]
pub struct InputContext<'eval_ctx, 'synth> {
	pub eval_ctx: &'eval_ctx EvaluationContext,
	pub value_store: &'synth Vec<f32>,
}

impl Input {
	pub(crate) fn evaluate(self, ctx: InputContext) -> f32 {
		// TODO: Profile, [f32]::get_unchecked
		match self {
			Input::Literal(f) => f,
			Input::Node(NodeID(idx)) => ctx.eval_ctx.sample_arena[idx as usize],
			Input::Store(StoreID(idx)) => ctx.value_store[idx as usize],
		}
	}
}

impl Into<Input> for f32 {
	fn into(self) -> Input { Input::Literal(self) }
}

impl Into<Input> for NodeID {
	fn into(self) -> Input { Input::Node(self) }
}

impl Into<Input> for StoreID {
	fn into(self) -> Input { Input::Store(self) }
}



#[derive(Debug, Clone, Copy)]
pub struct Phase {
	phase: f32,
	period: f32,

	freq: Input,
}

impl Phase {
	fn new(freq: Input) -> Phase {
		Phase {
			phase: 0.0,
			period: 1.0,

			freq
		}
	}

	fn with_period(freq: Input, period: f32) -> Phase {
		Phase {
			phase: 0.0,
			period,

			freq
		}
	}

	pub fn advance(&mut self, ctx: InputContext) -> f32 {
		let freq = self.freq.evaluate(ctx);

		self.phase += self.period * freq as f32 / ctx.eval_ctx.sample_rate as f32;
		self.phase %= self.period;
		self.phase as f32
	}
}



#[derive(Debug, Copy, Clone)]
pub struct NodeID (pub(crate) u32);

#[derive(Debug, Clone)]
pub enum Node {
	Sine{ phase: Phase },
	Triangle{ phase: Phase },
	Square{ phase: Phase },
	Saw{ phase: Phase },
	Noise{ phase: Phase },

	LowPass{ input: Input, freq: Input, prev_result: f32 },
	HighPass{ input: Input, freq: Input, prev_sample_diff: f32 },

	Clamp{ input: Input, lb: Input, ub: Input },
	Remap{ input: Input, in_lb: f32, in_ub: f32, out_lb: f32, out_ub: f32 },

	Mix{ a: Input, b: Input, mix: Input },
	Add(Input, Input),
	Subtract(Input, Input),
	Multiply(Input, Input),
	Divide(Input, Input),
	Power(Input, Input),

	StoreWrite(StoreID, Input),
	Sampler(BufferSampler),

	EnvAR(env::AR),
	EnvADSR(env::ADSR),
}

pub trait NodeContainer {
	fn add_node(&mut self, inst: Node) -> NodeID;

	
	fn new_sine<I: Into<Input>>(&mut self, freq: I) -> NodeID {
		self.add_node(Node::Sine{ phase: Phase::with_period(freq.into(), 2.0 * PI) })
	}

	fn new_triangle<I: Into<Input>>(&mut self, freq: I) -> NodeID {
		self.add_node(Node::Triangle{ phase: Phase::new(freq.into()) })
	}

	fn new_saw<I: Into<Input>>(&mut self, freq: I) -> NodeID {
		self.add_node(Node::Saw{ phase: Phase::new(freq.into()) })
	}

	fn new_square<I: Into<Input>>(&mut self, freq: I) -> NodeID {
		self.add_node(Node::Square{ phase: Phase::new(freq.into()) })
	}


	fn new_lowpass<I: Into<Input>, I2: Into<Input>>(&mut self, input: I, freq: I2) -> NodeID {
		self.add_node(Node::LowPass{ input: input.into(), freq: freq.into(), prev_result: 0.0 })
	}
	fn new_highpass<I: Into<Input>, I2: Into<Input>>(&mut self, input: I, freq: I2) -> NodeID {
		self.add_node(Node::HighPass{ input: input.into(), freq: freq.into(), prev_sample_diff: 0.0 })
	}


	fn new_clamp<I: Into<Input>, L: Into<Input>, U: Into<Input>>(&mut self, input: I, lb: L, ub: U) -> NodeID {
		self.add_node(Node::Clamp{input: input.into(), lb: lb.into(), ub: ub.into()})
	}

	fn new_remap<I: Into<Input>>(&mut self, input: I, in_lb: f32, in_ub: f32, out_lb: f32, out_ub: f32) -> NodeID {
		self.add_node(Node::Remap{ input: input.into(), in_lb, in_ub, out_lb, out_ub })
	}

	fn new_signal_to_control<I: Into<Input>>(&mut self, input: I) -> NodeID {
		self.new_remap(input, -1.0, 1.0,  0.0, 1.0)
	}

	fn new_control_to_signal<I: Into<Input>>(&mut self, input: I) -> NodeID {
		self.new_remap(input, 0.0, 1.0, -1.0, 1.0)
	}


	fn new_mix<A: Into<Input>, B: Into<Input>, M: Into<Input>>(&mut self, a: A, b: B, m: M) -> NodeID {
		self.add_node(Node::Mix{a: a.into(), b: b.into(), mix: m.into()})
	}

	fn new_add<I: Into<Input>, I2: Into<Input>>(&mut self, a: I, b: I2) -> NodeID { self.add_node(Node::Add(a.into(), b.into())) }
	fn new_sub<I: Into<Input>, I2: Into<Input>>(&mut self, a: I, b: I2) -> NodeID { self.add_node(Node::Subtract(a.into(), b.into())) }
	fn new_multiply<I: Into<Input>, I2: Into<Input>>(&mut self, a: I, b: I2) -> NodeID { self.add_node(Node::Multiply(a.into(), b.into())) }
	fn new_divide<I: Into<Input>, I2: Into<Input>>(&mut self, a: I, b: I2) -> NodeID { self.add_node(Node::Divide(a.into(), b.into())) }
	fn new_power<I: Into<Input>, I2: Into<Input>>(&mut self, a: I, b: I2) -> NodeID { self.add_node(Node::Power(a.into(), b.into())) }

	fn new_store_write<I: Into<Input>> (&mut self, store: StoreID, v: I) -> NodeID {
		self.add_node(Node::StoreWrite(store, v.into()))
	}
	fn new_sampler(&mut self, buffer_id: BufferID) -> NodeID {
		self.add_node(Node::Sampler(BufferSampler{buffer_id, position: 0}))
	}

	fn new_env_ar<G: Into<Input>> (&mut self, attack: f32, release: f32, gate: G) -> NodeID {
		self.add_node(Node::EnvAR(env::AR::new(attack, release, gate.into())))
	}
	fn new_env_adsr<G: Into<Input>> (&mut self, attack: f32, release: f32, gate: G) -> NodeID {
		unimplemented!()
		// self.add_node(Node::EnvADSR(env::ADSR::new(attack, release, gate.into())))
	}
}

impl NodeContainer for Synth {
	fn add_node(&mut self, inst: Node) -> NodeID {
		self.instructions.push(inst);
		NodeID(self.instructions.len() as u32 - 1)
	}
}
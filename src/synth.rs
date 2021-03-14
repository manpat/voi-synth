use crate::buffer::{Buffer, BufferID, BufferUsageType, SamplerContext};
use crate::context::EvaluationContext;
use crate::node::{Node, NodeID, InputContext};
use crate::parameter::{ParameterID, Parameter};

use crate::lerp;

use std::f32::consts::PI;
use std::sync::atomic;


static SYNTH_COUNTER: atomic::AtomicU32 = atomic::AtomicU32::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SynthID (pub(crate) u32);

#[derive(Copy, Clone, Debug)]
pub struct StoreID (pub(crate) u32);

#[derive(Clone, Debug)]
pub struct Synth {
	pub id: SynthID,

	gain: f32,
	output_node: Option<usize>,

	pub(crate) instructions: Vec<Node>,
	pub(crate) value_store: Vec<f32>,
	pub(crate) local_buffers: Vec<Buffer>,
	pub(crate) parameters: Vec<Parameter>,
}

macro_rules! input_context {
	($synth:expr, $eval_ctx:expr) => {{
		let eval_ctx = &$eval_ctx;
		let value_store = &$synth.value_store;
		let parameters = &$synth.parameters;

		InputContext { eval_ctx, value_store, parameters }
	}}
}

macro_rules! sampler_context {
	($synth:expr, $eval_ctx:expr) => {{
		let eval_ctx = &$eval_ctx;
		let local_buffers = &$synth.local_buffers;

		SamplerContext { eval_ctx, local_buffers }
	}}
}

impl Synth {
	pub fn new() -> Self {
		Synth {
			id: SynthID(SYNTH_COUNTER.fetch_add(1, atomic::Ordering::Relaxed)),

			gain: 1.0,
			output_node: None,

			instructions: Vec::new(),
			value_store: Vec::new(),
			local_buffers: Vec::new(),
			parameters: Vec::new(),
		}
	}

	pub fn new_value_store(&mut self) -> StoreID {
		self.value_store.push(0.0);
		StoreID(self.value_store.len() as u32 - 1)
	}

	pub fn new_parameter(&mut self) -> ParameterID {
		self.parameters.push(Parameter::new());
		ParameterID {
			owner: self.id,
			id: self.parameters.len() as u32 - 1
		}
	}

	pub fn get_parameter(&mut self, ParameterID{owner, id}: ParameterID) -> &mut Parameter {
		assert!(owner == self.id);
		&mut self.parameters[id as usize]
	}

	pub fn new_buffer(&mut self, data: Vec<f32>) -> BufferID {
		self.local_buffers.push(Buffer{ data });
		BufferID(BufferUsageType::Local, self.local_buffers.len() as u16 - 1)
	}

	pub fn set_gain(&mut self, gain: f32) { self.gain = gain }
	pub fn set_output(&mut self, NodeID(output): NodeID) { self.output_node = Some(output as usize); }

	pub fn evaluate_into_buffer(&mut self, buffer: &mut Buffer, eval_ctx: &mut EvaluationContext) {
		if eval_ctx.sample_arena.len() < self.instructions.len() {
			eval_ctx.sample_arena.resize(self.instructions.len(), 0.0);
		}

		for s in buffer.data.iter_mut() {
			let value = self.evaluate_sample(eval_ctx) * self.gain;
			*s += value;
		}
	}

	pub fn prewarm(&mut self, num_samples: usize, eval_ctx: &mut EvaluationContext) {
		if eval_ctx.sample_arena.len() < self.instructions.len() {
			eval_ctx.sample_arena.resize(self.instructions.len(), 0.0);
		}

		for _ in 0..num_samples { self.evaluate_sample(eval_ctx); }
	}


	fn evaluate_sample(&mut self, eval_ctx: &mut EvaluationContext) -> f32 {
		assert!(eval_ctx.sample_arena.len() >= self.instructions.len());

		for parameter in self.parameters.iter_mut() {
			parameter.update(eval_ctx);
		}

		let instructions = &mut self.instructions;

		for (idx, inst) in instructions.iter_mut().enumerate() {
			let sample = match inst {
				Node::Sine(phase) => phase.advance(input_context!(self, eval_ctx)).sin(),
				Node::Saw(phase) => phase.advance(input_context!(self, eval_ctx)) * 2.0 - 1.0,
				Node::Square(phase) => 1.0 - (phase.advance(input_context!(self, eval_ctx)) + 0.5).floor() * 2.0,
				Node::Triangle(phase) => {
					let ph = phase.advance(input_context!(self, eval_ctx));
					if ph <= 0.5 {
						(ph - 0.25)*4.0
					} else {
						(0.75 - ph)*4.0
					}
				}


				Node::LowPass{input, freq, prev_result} => {
					let ctx = input_context!(self, eval_ctx);
					let sample = input.evaluate(ctx);
					let cutoff = freq.evaluate(ctx);

					if cutoff > 0.0 {
						let dt = eval_ctx.sample_dt;
						let a = dt / (dt + 1.0 / (2.0 * PI * cutoff));
						*prev_result = lerp(*prev_result, sample, a);
					} else {
						*prev_result = 0.0;
					}
						
					*prev_result
				}

				Node::HighPass{input, freq, prev_sample_diff} => {
					let ctx = input_context!(self, eval_ctx);
					let sample = input.evaluate(ctx);
					let cutoff = freq.evaluate(ctx);

					let rc = 1.0 / (2.0 * PI * cutoff);
					let a = rc / (rc + eval_ctx.sample_dt);

					let result = a * (*prev_sample_diff + sample);
					*prev_sample_diff = result - sample;
						
					result
				}

				Node::Clamp{input, lb, ub} => {
					let ctx = input_context!(self, eval_ctx);
					let sample = input.evaluate(ctx);
					let lb_val = lb.evaluate(ctx);
					let ub_val = ub.evaluate(ctx);
					sample.max(lb_val).min(ub_val)
				}

				Node::Remap{input, in_lb, in_ub, out_lb, out_ub} => {
					let sample = input.evaluate(input_context!(self, eval_ctx));
					let normalised = (sample - *in_lb) / (*in_ub - *in_lb);
					normalised * (*out_ub - *out_lb) + *out_lb
				}

				Node::Mix{a, b, mix} => {
					let ctx = input_context!(self, eval_ctx);
					lerp(a.evaluate(ctx), b.evaluate(ctx), mix.evaluate(ctx))
				}

				Node::Add(a, b) => a.evaluate(input_context!(self, eval_ctx)) + b.evaluate(input_context!(self, eval_ctx)),
				Node::Subtract(a, b) => a.evaluate(input_context!(self, eval_ctx)) - b.evaluate(input_context!(self, eval_ctx)),
				Node::Multiply(a, b) => a.evaluate(input_context!(self, eval_ctx)) * b.evaluate(input_context!(self, eval_ctx)),
				Node::Divide(a, b) => a.evaluate(input_context!(self, eval_ctx)) / b.evaluate(input_context!(self, eval_ctx)),
				Node::Power(a, b) => a.evaluate(input_context!(self, eval_ctx)).powf(b.evaluate(input_context!(self, eval_ctx))),

				Node::StoreWrite(StoreID(idx), input) => {
					let v = input.evaluate(input_context!(self, eval_ctx));
					self.value_store[*idx as usize] = v;
					v
				}

				Node::Sampler{sampler, reset} => {
					if reset.update(input_context!(self, eval_ctx)).is_rising_edge() {
						sampler.reset();
					}

					sampler.sample(sampler_context!(self, eval_ctx))
				}

				Node::ParameterSampler(sampler) => {
					sampler.sample(&self.parameters)
				}

				Node::Sequencer{seq, advance, reset} => {
					let sample_ctx = sampler_context!(self, eval_ctx);
					let input_ctx = input_context!(self, eval_ctx);

					if advance.update(input_ctx).is_rising_edge() {
						seq.advance(sample_ctx);
					}

					if reset.update(input_ctx).is_rising_edge() {
						seq.reset();
					}

					seq.sample(sample_ctx)
				}

				Node::EnvAR(env_ar) => env_ar.advance(input_context!(self, eval_ctx)),
				Node::EnvADSR(env_adsr) => env_adsr.advance(input_context!(self, eval_ctx)),
			};

			unsafe {
				*eval_ctx.sample_arena.get_unchecked_mut(idx) = sample;
			}
		}

		if instructions.len() > 0 {
			let output_node = self.output_node.unwrap_or(instructions.len() - 1);

			unsafe {
				*eval_ctx.sample_arena.get_unchecked(output_node)
			}
		} else {
			0.0
		}
	}
}


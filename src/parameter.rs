use crate::context::EvaluationContext;
use crate::synth::SynthID;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct ParameterID {
	pub(crate) owner: SynthID,
	pub(crate) id: u32,
}

#[derive(Clone, Debug)]
pub struct Parameter {
	value: f32
}

impl Parameter {
	pub(crate) fn new() -> Self {
		Parameter { value: 0.0 }
	}

	pub(crate) fn update(&mut self, eval_ctx: &EvaluationContext) {
		// interpolation
	}

	pub(crate) fn evaluate(&self) -> f32 {
		self.value
	}

	pub fn set_value(&mut self, val: f32) { self.value = val; }
}


#[derive(Clone, Debug)]
pub enum SampleMode {
	Linear
}


#[derive(Clone, Debug)]
pub struct ParameterSampler {
	parameter: ParameterID,
	sample_mode: SampleMode,
}

impl ParameterSampler {
	pub(crate) fn new(parameter: ParameterID, sample_mode: SampleMode) -> Self {
		ParameterSampler {
			parameter, sample_mode
		}
	}

	pub(crate) fn sample(&mut self, parameters: &[Parameter]) -> f32 {
		let new_sample = parameters[self.parameter.id as usize].evaluate();
		new_sample
		// TODO
	}
}
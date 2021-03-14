use crate::node::{Input, InputContext};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GateState { Low, RisingEdge, High, FallingEdge }

#[derive(Clone, Debug)]
pub struct Gate(Input, GateState, f32);

// TODO: configurable
const GATE_THRESHOLD: f32 = 0.05;

impl Gate {
	pub fn new(input: Input) -> Self { Gate (input, GateState::Low, 0.0) }

	pub fn update(&mut self, ctx: InputContext) -> GateState {
		use self::GateState::*;

		let input_sample = self.0.evaluate(ctx);
		let diff = input_sample - self.2;
		self.2 = input_sample;

		self.1 = match self.1 {
			Low => {
				if input_sample > GATE_THRESHOLD { RisingEdge }
				else { Low }
			}

			RisingEdge => {
				if input_sample >= GATE_THRESHOLD { High }
				else { FallingEdge }
			}

			High => {
				if input_sample < GATE_THRESHOLD { FallingEdge }
				else { High }
			}

			FallingEdge => {
				if input_sample <= GATE_THRESHOLD { Low }
				else { RisingEdge }
			}
		};

		self.1
	}
}


impl GateState {
	pub fn is_rising_edge(self) -> bool {
		match self {
			GateState::RisingEdge => true,
			_ => false
		}
	}
	pub fn is_falling_edge(self) -> bool {
		match self {
			GateState::FallingEdge => true,
			_ => false
		}
	}
	pub fn is_highish(self) -> bool {
		match self {
			GateState::RisingEdge => true,
			GateState::High => true,
			_ => false
		}
	}
	pub fn is_lowish(self) -> bool {
		match self {
			GateState::FallingEdge => true,
			GateState::Low => true,
			_ => false
		}
	}
}
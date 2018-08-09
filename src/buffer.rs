use context::EvaluationContext;

#[derive(Clone, Copy, Debug)]
pub(crate) enum BufferUsageType {
	Local, Shared,
}

#[derive(Clone, Copy, Debug)]
pub struct BufferID(pub(crate) BufferUsageType, pub(crate) u16);

#[derive(Clone, Debug)]
pub struct Buffer { pub data: Vec<f32> }

impl Buffer {
	pub fn new(buffer_size: usize) -> Buffer {
		Buffer{ data: vec![0.0; buffer_size] }
	}

	pub fn clear(&mut self) {
		for v in self.data.iter_mut() { *v = 0.0; }
	}

	pub fn len(&self) -> usize { self.data.len() } 

	pub unsafe fn copy_to(&self, dst: *mut u8, length: usize) {
		use std::mem::transmute;
		use std::ptr;

		let dst = transmute(dst);
		ptr::copy(self.data.as_ptr(), dst, self.data.len().min(length / 4));
	}
}

#[derive(Clone, Copy)]
pub struct SamplerContext<'eval_ctx, 'synth> {
	pub eval_ctx: &'eval_ctx EvaluationContext,
	pub local_buffers: &'synth Vec<Buffer>,
}

#[derive(Clone, Debug)]
pub struct BufferSampler { pub buffer_id: BufferID, pub position: usize }

impl BufferSampler {
	pub fn advance(&mut self, ctx: SamplerContext) -> f32 {
		use self::BufferUsageType::*;

		let BufferID(usage, id) = self.buffer_id;
		let idx = id as usize;

		let buffer = match usage {
			Local => &ctx.local_buffers[idx],
			Shared => &ctx.eval_ctx.shared_buffers[idx],
		};

		let num_samples = buffer.len();

		assert!(self.position < num_samples);

		let sample = buffer.data[self.position];
		self.position = (self.position + 1) % num_samples;
		sample
	}
}
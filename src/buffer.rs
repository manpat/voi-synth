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

	pub fn resize(&mut self, buffer_size: usize) {
		self.data.resize(buffer_size, 0.0);
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

	pub unsafe fn copy_to_stereo(&self, dst: *mut u8, length: usize) {
		use std::mem::transmute;
		use std::mem::size_of;

		type SampleType = [f32; 2];

		let stereo = self.data.iter()
			.map(|f| [*f, *f])
			.take(length / size_of::<SampleType>());

		let dst: *mut SampleType = transmute(dst);
		for (i, s) in stereo.enumerate() {
			*dst.offset(i as isize) = s;
		}
	}
}



#[derive(Clone, Copy)]
pub struct SamplerContext<'eval_ctx, 'synth> {
	pub eval_ctx: &'eval_ctx EvaluationContext,
	pub local_buffers: &'synth Vec<Buffer>,
}

impl<'e,'s> SamplerContext<'e,'s> {
	fn get_buffer(&self, BufferID(usage, id): BufferID) -> &Buffer {
		use self::BufferUsageType::*;

		let idx = id as usize;

		match usage {
			Local => &self.local_buffers[idx],
			Shared => &self.eval_ctx.shared_buffers[idx],
		}
	}
}



#[derive(Clone, Debug)]
pub struct Sequencer { pub buffer_id: BufferID, pub position: usize }

impl Sequencer {
	pub fn new(buffer_id: BufferID) -> Self {
		Sequencer {
			buffer_id, position: 0
		}
	} 

	pub fn reset(&mut self) {
		self.position = 0;
	}

	pub fn advance(&mut self, ctx: SamplerContext) {
		let buffer = ctx.get_buffer(self.buffer_id);
		let num_samples = buffer.len();
		self.position = (self.position + 1) % num_samples;
	}

	pub fn sample(&mut self, ctx: SamplerContext) -> f32 {
		let buffer = ctx.get_buffer(self.buffer_id);
		let num_samples = buffer.len();

		assert!(self.position < num_samples);

		buffer.data[self.position]
	}
}




#[derive(Clone, Debug)]
pub struct BufferSampler(pub Sequencer);

impl BufferSampler {
	pub fn new(buffer_id: BufferID) -> Self {
		BufferSampler(Sequencer::new(buffer_id))
	}

	pub fn reset(&mut self) { self.0.reset(); }

	pub fn sample(&mut self, ctx: SamplerContext) -> f32 {
		let seq = &mut self.0;
		let sample = seq.sample(ctx);
		seq.advance(ctx);
		sample
	}
}
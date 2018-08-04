use std::thread::spawn;
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::sync::{Mutex, Arc};
use SynthResult;
use synth::Synth;
use buffer::Buffer;

use failure::err_msg;

use lerp;

pub struct Context {
	shared_context: Arc<Mutex<SharedContext>>,

	queued_buffer_tx: SyncSender<Buffer>,
	ready_buffer_rx: Receiver<Buffer>,
}

impl Context {
	pub fn new() -> Self {
		let (queued_buffer_tx, queued_buffer_rx) = sync_channel::<Buffer>(16);
		let (ready_buffer_tx, ready_buffer_rx) = sync_channel::<Buffer>(16);

		let shared_context = Arc::new(Mutex::new(SharedContext::new()));

		{
			let shared_context = shared_context.clone();

			spawn::<_, SynthResult<_>>(move || {
				for mut buffer in queued_buffer_rx.iter() {
					shared_context.lock().map_err(
						|_| err_msg("Failed to lock shared context in evaluation thread"))?
						.fill_buffer(&mut buffer);

					ready_buffer_tx.send(buffer)?;
				}
	
				Ok(())
			});
		}

		Context {
			shared_context,

			queued_buffer_tx,
			ready_buffer_rx,
		}
	}

	pub fn dump_stats(&self) {
		let ctx = self.shared_context.lock().unwrap();

		let max_micros_per_buffer = 1024.0 / ctx.evaluation_ctx.sample_rate * 100000.0;

		println!("fill time: {:5.0}μs / {} synths = {:3.2}μs/synth   (limit: {:5.0}μs, rem:{:5.0}μs)    dc: {:1.6}, env: {}",
			ctx.average_fill_time,
			ctx.synths.len(),
			ctx.average_fill_time / ctx.synths.len() as f32,
			max_micros_per_buffer,
			max_micros_per_buffer - ctx.average_fill_time,
			ctx.signal_dc, ctx.envelope);
	}

	pub fn push_synth(&self, mut synth: Synth) -> SynthResult<u32> {
		let mut ctx = self.shared_context.lock().unwrap();

		let id = ctx.synths.len() as u32;
		synth.id = id;
		ctx.synths.push(synth);
		Ok(id)
	}

	pub fn set_sample_rate(&self, sample_rate: f32) {
		self.shared_context.lock().unwrap().evaluation_ctx.sample_rate = sample_rate;
	}

	pub fn init_buffer_queue(&self, buffer_size: usize, buffer_count: usize) -> SynthResult<()> {
		for _ in 0..buffer_count {
			self.queued_buffer_tx.send(Buffer::new(buffer_size))?;
		}

		Ok(())
	}

	pub fn get_ready_buffer(&self) -> SynthResult<Buffer> {
		Ok(self.ready_buffer_rx.recv()?)
	}

	pub fn queue_empty_buffer(&self, buffer: Buffer) -> SynthResult<()> {
		Ok(self.queued_buffer_tx.send(buffer)?)
	}
}


pub struct EvaluationContext {
	pub sample_rate: f32,

	pub sample_arena: Vec<f32>,

	// Wavetables
}



struct SharedContext {
	synths: Vec<Synth>,
	// interpolators: Vec<Interpolator>,
	// command_rx: Reciever<SynthCommand>,

	envelope: f32,
	signal_dc: f32,

	evaluation_ctx: EvaluationContext,

	average_fill_time: f32,
}

impl SharedContext {
	fn new() -> Self {
		// Init wavetables

		SharedContext {
			synths: Vec::new(),
			envelope: 1000.0,
			signal_dc: 0.0,

			evaluation_ctx: EvaluationContext {
				sample_rate: 22050.0,
				sample_arena: Vec::new(),
			},

			average_fill_time: 0.0,
		}
	}

	fn fill_buffer(&mut self, buffer: &mut Buffer) {
		use std::time;

		let begin = time::Instant::now();

		buffer.clear();

		for synth in self.synths.iter_mut() {
			synth.evaluate_into_buffer(buffer, &mut self.evaluation_ctx);
		}

		const ATTACK_TIME: f32  = 5.0 / 1000.0;
		const RELEASE_TIME: f32 = 200.0 / 1000.0;

		let sample_rate = self.evaluation_ctx.sample_rate;

		let attack  = 1.0 - (-1.0 / (ATTACK_TIME * sample_rate)).exp();
		let release = 1.0 - (-1.0 / (RELEASE_TIME * sample_rate)).exp();

		for v in buffer.data.iter_mut() {
			let mut sample = *v;
			self.signal_dc = lerp(self.signal_dc, sample, 0.5/sample_rate);
			sample -= self.signal_dc;

			let abs_signal = sample.abs();
			if abs_signal > self.envelope {
				self.envelope = lerp(self.envelope, abs_signal, attack);
			} else {
				self.envelope = lerp(self.envelope, abs_signal, release);
			}

			self.envelope = self.envelope.max(1.0);
			*v = (sample*0.6/self.envelope).max(-1.0).min(1.0);
		}

		let end = time::Instant::now();
		let diff = (end-begin).subsec_nanos() as f32 / 1000.0;

		self.average_fill_time = lerp(self.average_fill_time, diff, 0.01);
	}
}

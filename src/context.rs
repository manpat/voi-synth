use std::thread::spawn;
use std::sync::mpsc::{SyncSender, Sender, Receiver, sync_channel, channel};
use std::sync::{Mutex, Arc};

use crate::SynthResult;
use crate::synth::{Synth, SynthID};
use crate::buffer::{Buffer, BufferID, BufferUsageType};
use crate::parameter::ParameterID;

use crate::lerp;

use failure::err_msg;

pub struct Context {
	shared_context: Arc<Mutex<SharedContext>>,

	event_tx: Sender<SynthEvent>,

	queued_buffer_tx: SyncSender<Buffer>,
	ready_buffer_rx: Receiver<Buffer>,
	buffer_size: usize,
}

impl Context {
	pub fn new(buffer_count: usize, buffer_size: usize) -> SynthResult<Self> {
		let (queued_buffer_tx, queued_buffer_rx) = sync_channel::<Buffer>(16);
		let (ready_buffer_tx, ready_buffer_rx) = sync_channel::<Buffer>(16);

		let (event_tx, event_rx) = channel();

		let shared_context = Arc::new(Mutex::new(SharedContext::new(event_rx)));

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

		for _ in 0..buffer_count {
			queued_buffer_tx.send(Buffer::new(buffer_size))?;
		}

		Ok(Context {
			shared_context,

			event_tx,

			queued_buffer_tx,
			ready_buffer_rx,

			buffer_size,
		})
	}

	pub fn dump_stats(&self) {
		let ctx = self.shared_context.lock().unwrap();

		let max_micros_per_buffer = 256.0 / ctx.evaluation_ctx.sample_rate * 1000000.0;

		println!("fill time: {:5.0}μs / {} synths = {:3.2}μs/synth   (limit: {:5.0}μs, rem:{:5.0}μs)    dc: {:1.6}, env: {}",
			ctx.average_fill_time,
			ctx.synths.len(),
			ctx.average_fill_time / ctx.synths.len() as f32,
			max_micros_per_buffer,
			max_micros_per_buffer - ctx.average_fill_time,
			ctx.signal_dc, ctx.envelope);
	}

	pub fn push_synth(&self, mut synth: Synth) -> SynthResult<SynthID> {
		let mut ctx = self.shared_context.lock().unwrap();

		let id = synth.id;
		ctx.synths.push(synth);
		Ok(id)
	}

	pub fn remove_synth(&self, synth_id: SynthID) {
		let mut ctx = self.shared_context.lock().unwrap();
		ctx.synths.retain(move |s| s.id != synth_id);
	}

	pub fn set_sample_rate(&self, sample_rate: f32) {
		let mut ctx = self.shared_context.lock().unwrap();
		ctx.evaluation_ctx.sample_rate = sample_rate;
		ctx.evaluation_ctx.sample_dt = 1.0 / sample_rate;
	}

	pub fn get_sample_rate(&self) -> f32 {
		let ctx = self.shared_context.lock().unwrap();
		ctx.evaluation_ctx.sample_rate
		// TODO: find a way to remove this
	}

	pub fn set_buffer_size(&mut self, buffer_size: usize) {
		self.buffer_size = buffer_size;
	}

	pub fn get_buffer_size(&self) -> usize {
		self.buffer_size
	}

	pub fn set_parameter(&self, param_id: ParameterID, value: f32) {
		self.event_tx.send(SynthEvent::SetParam(param_id, value)).unwrap();
	}

	pub fn create_shared_buffer(&self, data: Vec<f32>) -> SynthResult<BufferID> {
		let mut ctx = self.shared_context.lock().unwrap();
		ctx.evaluation_ctx.shared_buffers.push(Buffer{ data });
		Ok(BufferID(BufferUsageType::Shared, (ctx.evaluation_ctx.shared_buffers.len() - 1) as u16))
	}

	pub fn get_ready_buffer(&self) -> SynthResult<Buffer> {
		Ok(self.ready_buffer_rx.recv()?)
	}

	pub fn queue_empty_buffer(&self, mut buffer: Buffer) -> SynthResult<()> {
		buffer.resize(self.buffer_size);
		Ok(self.queued_buffer_tx.send(buffer)?)
	}
}


enum SynthEvent {
	SetParam(ParameterID, f32),
	// NewSynth
	// NewSharedBuffer
	// SampleRateChange
	// BufferSizeChange
}


pub struct EvaluationContext {
	pub sample_rate: f32,
	pub sample_dt: f32,

	pub sample_arena: Vec<f32>,
	pub shared_buffers: Vec<Buffer>,

	// Wavetables
}

impl EvaluationContext {
	pub fn new(sample_rate: f32) -> Self {
		EvaluationContext {
			sample_rate,
			sample_dt: 1.0 / sample_rate, 

			sample_arena: Vec::new(),
			shared_buffers: Vec::new(),
		}
	}
}



struct SharedContext {
	synths: Vec<Synth>,
	// interpolators: Vec<Interpolator>,
	event_rx: Receiver<SynthEvent>,

	envelope: f32,
	signal_dc: f32,

	evaluation_ctx: EvaluationContext,

	average_fill_time: f32,
}

impl SharedContext {
	fn new(event_rx: Receiver<SynthEvent>) -> Self {
		// Init wavetables

		SharedContext {
			synths: Vec::new(),
			event_rx,

			envelope: 1000.0,
			signal_dc: 0.0,

			evaluation_ctx: EvaluationContext::new(22050.0),

			average_fill_time: 0.0,
		}
	}

	fn fill_buffer(&mut self, buffer: &mut Buffer) {
		use std::time;

		let begin = time::Instant::now();

		buffer.clear();

		for ev in self.event_rx.try_recv() {
			if let SynthEvent::SetParam(param_id, value) = ev {
				let param = self.synths.iter_mut()
					.find(|s| s.id == param_id.owner)
					.map(move |s| s.get_parameter(param_id));

				if let Some(param) = param {
					param.set_value(value);
				}
			}
		}


		// TODO: multiple evaluation contexts, push synth evals to diff threads
		// recombine on completion
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

		self.average_fill_time = lerp(self.average_fill_time, diff, 0.1);
	}
}

use std::thread::{JoinHandle, spawn};
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::sync::{Mutex, Arc};
use SynthResult;
use synth::Synth;
use buffer::Buffer;

use failure::err_msg;

pub struct Context {
	shared_context: Arc<Mutex<SharedContext>>,

	evaluation_thread: JoinHandle<SynthResult<()>>,
	queued_buffer_tx: SyncSender<Buffer>,
	ready_buffer_rx: Receiver<Buffer>,
}

impl Context {
	pub fn new() -> Self {
		let (queued_buffer_tx, queued_buffer_rx) = sync_channel::<Buffer>(16);
		let (ready_buffer_tx, ready_buffer_rx) = sync_channel::<Buffer>(16);

		let shared_context = Arc::new(Mutex::new(SharedContext::new()));
		{
			let mut ctx = shared_context.lock().unwrap();
			let freqs = [
				55.0, 56.0, 57.0,
				110.0, 110.1, 110.2, 110.3,
				220.0, 220.5,
				220.0 * 3.0/4.0,
				220.1 * 3.0/4.0,
				220.2 * 3.0/4.0,
				220.3 * 3.0/4.0,
				330.2, 330.9, 331.1,
				331.2, 331.9, 332.1,
				440.0, 440.3, 440.7,
				550.0, 550.3, 550.7,
				660.0, 660.3, 660.7,
				770.0, 770.3, 770.7,
				880.0, 880.3, 880.7,
			];

			for &f in freqs.iter() {
				ctx.synths.push(Synth::with_freq(f));
			}
		}

		let evaluation_thread = {
			let shared_context = shared_context.clone();

			spawn(move || {
				for mut buffer in queued_buffer_rx.iter() {
					let mut ctx = shared_context.lock().map_err(
						|_| err_msg("Failed to lock shared context in evaluation thread"))?;

					ctx.fill_buffer(&mut buffer);
					ready_buffer_tx.send(buffer)?;
				}
	
				Ok(())
			})
		};

		Context {
			shared_context,

			evaluation_thread,
			queued_buffer_tx,
			ready_buffer_rx,
		}
	}

	pub fn init_buffer_queue(&mut self, buffer_size: usize, buffer_count: usize) -> SynthResult<()> {
		for _ in 0..buffer_count {
			self.queued_buffer_tx.send(Buffer::new(buffer_size))?;
		}

		Ok(())
	}

	pub fn get_ready_buffer(&mut self) -> SynthResult<Buffer> {
		Ok(self.ready_buffer_rx.recv()?)
	}

	pub fn queue_empty_buffer(&mut self, buffer: Buffer) -> SynthResult<()> {
		Ok(self.queued_buffer_tx.send(buffer)?)
	}
}


struct SharedContext {
	synths: Vec<Synth>,
	envelope: f32,
	signal_dc: f32,

	average_fill_time: f32,
}

impl SharedContext {
	fn new() -> Self {
		SharedContext {
			synths: Vec::new(),
			envelope: 0.0,
			signal_dc: 0.0,

			average_fill_time: 0.0,
		}
	}

	fn fill_buffer(&mut self, buffer: &mut Buffer) {
		use std::time;

		let begin = time::Instant::now();

		buffer.clear();

		for synth in self.synths.iter_mut() {
			synth.evaluate_into_buffer(buffer);
		}

		let sample_rate = 22050.0;
		const ATTACK_TIME: f32  = 5.0 / 1000.0;
		const RELEASE_TIME: f32 = 200.0 / 1000.0;

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
			*v = (sample*0.7/self.envelope).max(-1.0).min(1.0);
		}

		let end = time::Instant::now();
		let diff = (end-begin).subsec_nanos() as f32 / 1000.0;

		self.average_fill_time = lerp(self.average_fill_time, diff, 0.01);

		println!("fill time: {:5.0}μs (avg {:5.0}μs, {:3.0}μs/synth)",
			diff, self.average_fill_time,
			self.average_fill_time / self.synths.len() as f32);
	}
}

fn lerp(from: f32, to: f32, amt: f32) -> f32 {
	from + (to-from) * amt
}
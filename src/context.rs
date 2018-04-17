use std::thread::{JoinHandle, spawn};
use std::sync::mpsc::{SyncSender, Receiver, sync_channel};
use std::sync::{Mutex, Arc};
use SynthResult;
use synth::Synth;

use failure::err_msg;

pub struct Buffer {}

pub struct Context {
	shared_context: Arc<Mutex<SharedContext>>,

	evaluation_thread: JoinHandle<SynthResult<()>>,
	queued_buffer_tx: SyncSender<Buffer>,
	ready_buffer_rx: Receiver<Buffer>,

	ready_buffers: Vec<Buffer>,
}

impl Context {
	pub fn new() -> Self {
		let (queued_buffer_tx, queued_buffer_rx) = sync_channel::<Buffer>(16);
		let (ready_buffer_tx, ready_buffer_rx) = sync_channel::<Buffer>(16);

		let shared_context = Arc::new(Mutex::new(SharedContext::new()));

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

			ready_buffers: Vec::new(),
		}
	}

	pub fn update(&mut self) {
		self.ready_buffers.extend(self.ready_buffer_rx.try_iter());
	} 
}


struct SharedContext {
	synths: Vec<Synth>,
}

impl SharedContext {
	fn new() -> Self {
		SharedContext {
			synths: Vec::new(),
		}
	}

	fn fill_buffer(&mut self, buffer: &mut Buffer) {

	}
}
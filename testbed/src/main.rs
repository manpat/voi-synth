#![feature(box_syntax)]

extern crate voi_synth;
extern crate sdl2_sys as sdl;
#[macro_use] extern crate failure;

use failure::Error;
use voi_synth::*;

macro_rules! cstr {
	($str:expr) => {{
		use std::ffi::CString;
		CString::new($str).unwrap().as_bytes_with_nul().as_ptr() as _
	}};
}

macro_rules! from_cstr {
	($str:expr) => {{
		use std::ffi::CStr;
		CStr::from_ptr($str as _).to_str().unwrap()
	}};
}


mod window;

use window::*;

fn main() {
	let mut window = Window::new().expect("Window open failed");
	let mut synth_context = voi_synth::Context::new();

	let mut audio_device = init_audio(&mut synth_context).expect("Audio init failed");
	synth_context.init_buffer_queue(audio_device.buffer_size, 3)
		.expect("Failed to init audio buffer queue");
	start_audio(&mut audio_device);

	'main_loop: loop {
		for (evt_ty, event) in EventIter {
			use sdl::SDL_EventType::*;
			use sdl::*;

			match evt_ty {
				SDL_QUIT => break 'main_loop,
				SDL_KEYDOWN => unsafe {
					if event.key.keysym.sym == SDLK_ESCAPE as i32 {
						break 'main_loop
					}
				}
				_ => {}
			}
		}

		window.swap();
	}
}

struct AudioCtx {
	device_id: sdl::SDL_AudioDeviceID,
	sample_freq: u32,
	buffer_size: usize,

	synth_context: *mut voi_synth::Context,
}

fn init_audio(synth_context: &mut voi_synth::Context) -> SynthResult<Box<AudioCtx>> {
	use std::mem::{zeroed, uninitialized, transmute};
	use std::ptr::null;
	use sdl::*;

	let mut audio_ctx = box AudioCtx {
		device_id: 0,
		sample_freq: 0,
		buffer_size: 0,
		synth_context: unsafe{ transmute(synth_context) },
	};

	let mut want: SDL_AudioSpec = unsafe { zeroed() };
	let mut have: SDL_AudioSpec = unsafe { uninitialized() };

	want.freq = 22050;
	want.format = AUDIO_F32SYS as u16;
	want.channels = 2;
	want.samples = 256;
	want.callback = Some(audio_callback);
	want.userdata = unsafe{ transmute(&mut *audio_ctx) };

	let dev = unsafe {
		SDL_OpenAudioDevice(null(), 0, &want, &mut have, SDL_AUDIO_ALLOW_FREQUENCY_CHANGE as i32)
	};
	
	ensure!(dev != 0, "Failed to open audio: {}", unsafe { from_cstr!(SDL_GetError()) } );
	ensure!(have.channels == 2, "Failed to get stereo audio");
	ensure!(have.format == AUDIO_F32SYS as _, "Failed to get wanted output format");

	// Init wavetables

	audio_ctx.device_id = dev;
	audio_ctx.sample_freq = have.freq as u32;
	audio_ctx.buffer_size = have.samples as usize * have.channels as usize;

	Ok(audio_ctx)
}

fn start_audio(audio_ctx: &mut Box<AudioCtx>) {
	unsafe {
		sdl::SDL_PauseAudioDevice(audio_ctx.device_id, 0);
	}
}

unsafe extern fn audio_callback(ud: *mut std::os::raw::c_void, stream: *mut u8, length: i32) {
	use std::mem::transmute;

	let audio_ctx: &mut AudioCtx = transmute(ud);
	let synth_context: &mut voi_synth::Context = transmute(audio_ctx.synth_context);

	let buffer = synth_context.get_ready_buffer().expect("Failed to get ready buffer");

	buffer.copy_to(stream, length as usize);

	synth_context.queue_empty_buffer(buffer).unwrap();
}
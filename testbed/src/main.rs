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

	let _audio_device = init_audio().expect("Audio init failed");

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

		synth_context.update();

		window.swap();
	}
}

struct AudioCtx {
	device_id: sdl::SDL_AudioDeviceID,
	sample_freq: u32,
}

fn init_audio() -> SynthResult<AudioCtx> {
	use std::mem::{zeroed, uninitialized};
	use std::ptr::null;
	use sdl::*;

	let mut want: SDL_AudioSpec = unsafe { zeroed() };
	let mut have: SDL_AudioSpec = unsafe { uninitialized() };

	want.freq = 22050;
	want.format = AUDIO_F32SYS as u16;
	want.channels = 2;
	want.samples = 256;
	want.callback = Some(audio_callback);

	let dev = unsafe {
		SDL_OpenAudioDevice(null(), 0, &want, &mut have, SDL_AUDIO_ALLOW_FREQUENCY_CHANGE as i32)
	};
	
	ensure!(dev != 0, "Failed to open audio: {}", unsafe { from_cstr!(SDL_GetError()) } );
	ensure!(have.channels == 2, "Failed to get stereo audio");

	// Init wavetables
	// Unpause device

	Ok(AudioCtx{
		device_id: dev,
		sample_freq: have.freq as u32
	})
}

unsafe extern fn audio_callback(ud: *mut std::os::raw::c_void, stream: *mut u8, length: i32) {

}
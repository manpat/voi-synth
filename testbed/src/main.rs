#![feature(box_syntax)]
#![feature(nll)]

extern crate voi_synth;
extern crate sdl2_sys as sdl;
#[macro_use] extern crate failure;

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

use voi_synth::*;
use window::*;

fn main() {
	let _window = Window::new().expect("Window open failed");
	let mut synth_context = box voi_synth::Context::new();

	// let freqs = [
	// 	55.0, 56.0, 57.0,
	// 	110.0, 110.1, 110.2, 110.3,
	// 	220.0,
	// 	220.5,
	// 	220.0 * 3.0/4.0,
	// 	220.1 * 3.0/4.0,
	// 	220.2 * 3.0/4.0,
	// 	220.3 * 3.0/4.0,
	// 	330.2, 330.9, 331.1,
	// 	331.2, 331.9, 332.1,
	// 	440.0, 440.3, 440.7,
	// 	550.0, 550.3, 550.7,
	// 	660.0, 660.3, 660.7,
	// 	770.0, 770.3, 770.7,
	// 	880.0, 880.3, 880.7,
	// ];

	// for i in 0..1 {
	// 	for &f in freqs.iter() {
	// 		use voi_synth::synth::*;

	// 		let mut synth = Synth::new();
	// 		synth.set_gain(0.02);

	// 		let f = f + (i as f32) * 0.05;

	// 		match i%4 {
	// 			0 => { synth.push_node(Node::new_sine(f)); }
	// 			1 => { synth.push_node(Node::new_triangle(f)); }
	// 			2 => { synth.push_node(Node::new_saw(f)); }
	// 			3 => { synth.push_node(Node::new_square(f)); }

	// 			_ => {}
	// 		}

	// 		synth_context.push_synth(synth).unwrap();
	// 	}
	// }

	// {
	// 	let mut synth = Synth::new();
	// 	synth.set_gain(0.3);

	// 	let feedback = synth.new_value_store();

	// 	let lfo_osc = synth.new_sine(1.0/3.4);
	// 	let lfo = synth.new_multiply(lfo_osc, 2.0);

	// 	let freq = synth.new_add(lfo, 110.0);
	// 	let freq_2 = synth.new_multiply(freq, 3.02 / 2.0);
	// 	let freq_bass = synth.new_multiply(freq, 1.0/2.0);

	// 	let feedback_add = synth.new_multiply(freq, feedback);
	// 	let a = synth.new_saw(feedback_add);
	// 	let b = synth.new_saw(freq_2);
	// 	let c = synth.new_add(a, b);

	// 	let bass = synth.new_sine(freq_bass);

	// 	let wobble_osc = synth.new_sine(6.0);
	// 	let wobble = synth.new_remap(wobble_osc, -1.0, 1.0,  0.0, 1.0);
	// 	let wobble = synth.new_power(wobble, 5.0);
	// 	let wobble_freq = synth.new_remap(wobble, 0.0, 1.0,  600.0, 20000.0);

	// 	let lp = synth.new_lowpass(c, wobble_freq);
	// 	let lp = synth.new_lowpass(lp, wobble_freq);
	// 	let lp = synth.new_lowpass(lp, wobble_freq);
	// 	let lp = synth.new_lowpass(lp, wobble_freq);

	// 	let lfo_vc = synth.new_remap(wobble_osc, -1.0, 1.0,  0.8, 1.2);
	// 	let feedback_mul = synth.new_multiply(4.0 / 3.0, lfo_vc);
	// 	let feedback_mul = synth.new_multiply(lp, feedback_mul);
	// 	synth.new_store_write(feedback, feedback_mul);

	// 	synth.new_add(c, bass);

	// 	synth_context.push_synth(synth).unwrap();
	// }

	{
		let mut synth = Synth::new();
		synth.set_gain(0.5);

		let mut feedback_chain = Vec::new();

		for _ in 0..64 {
			feedback_chain.push(synth.new_value_store());
		}

		let feedback_head = feedback_chain[0];
		let feedback_tail = feedback_chain[feedback_chain.len() - 1];

		let mul_osc = synth.new_multiply(52.0, feedback_tail);
		// let mul_osc = synth.new_multiply(80.0, feedback_tail);
		// let mul_osc = synth.new_multiply(110.0, feedback_tail);

		let fm = synth.new_saw(mul_osc);
		let fm = synth.new_multiply(fm, 180.0);

		let oscf0 = synth.new_add(220.0, fm);
		let oscf1 = synth.new_multiply(oscf0, 0.51);
		let oscf2 = synth.new_multiply(oscf0, 2.0);

		let osc = synth.new_triangle(oscf0);
		let osc2 = synth.new_square(oscf1);
		let osc3 = synth.new_sine(oscf2);

		let osc = synth.new_add(osc, osc2);
		let osc = synth.new_add(osc, osc3);

		// let osc = synth.new_clamp(osc, -100.0, 1.0);
		let mul_osc = synth.new_sub(osc, feedback_tail);
		// let mul_osc = synth.new_sub(feedback_tail, osc);
		// let mul_osc_lfo = synth.new_square(2.0);
		let mul_osc_lfo = synth.new_triangle(200.0);
		let mul_osc_lfo = synth.new_signal_to_control(mul_osc_lfo);
		let mul_osc_lfo = synth.new_power(mul_osc_lfo, 5.0);
		let mul_osc_lfo = synth.new_control_to_signal(mul_osc_lfo);
		let mul_osc = synth.new_multiply(mul_osc, mul_osc_lfo);

		for sd in feedback_chain.windows(2).rev() {
			if let &[src, dst] = sd {
				synth.new_store_write(dst, src);
			}
		}

		synth.new_store_write(feedback_head, mul_osc);
		synth.set_output(osc);

		synth_context.push_synth(synth).unwrap();
	}

	{
		let mut synth = Synth::new();
		synth.set_gain(200.0);

		let beat = synth.new_square(1.0);
		let env = synth.new_env_ar(0.08, 0.76, beat);

		let freq_mod = synth.new_multiply(env, 10.0);
		let freq = synth.new_add(25.0, freq_mod);
		let osc = synth.new_triangle(freq);

		let mixed = synth.new_multiply(osc, env);
		synth.set_output(mixed);

		synth_context.push_synth(synth).unwrap();
	}

	let mut audio_device = init_audio(&mut synth_context).expect("Audio init failed");
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

		synth_context.dump_stats();

		use std::thread::sleep;
		use std::time::Duration;
		sleep(Duration::from_millis(32));
	}

	stop_audio(&mut audio_device);
}

struct AudioCtx {
	device_id: sdl::SDL_AudioDeviceID,
}

fn init_audio(synth_context: &mut Box<voi_synth::Context>) -> SynthResult<AudioCtx> {
	use std::mem::{zeroed, uninitialized, transmute};
	use std::ptr::null;
	use sdl::*;

	let mut want: SDL_AudioSpec = unsafe { zeroed() };
	let mut have: SDL_AudioSpec = unsafe { uninitialized() };

	// want.freq = 22050;
	want.freq = 44100;
	want.format = AUDIO_F32SYS as u16;
	want.channels = 2;
	want.samples = 256;
	want.callback = Some(audio_callback);
	want.userdata = unsafe{ transmute(&mut **synth_context) };

	let device_id = unsafe {
		SDL_OpenAudioDevice(null(), 0, &want, &mut have, SDL_AUDIO_ALLOW_FREQUENCY_CHANGE as i32)
	};
	
	ensure!(device_id != 0, "Failed to open audio: {}", unsafe { from_cstr!(SDL_GetError()) } );
	ensure!(have.channels == 2, "Failed to get stereo audio");
	ensure!(have.format == AUDIO_F32SYS as _, "Failed to get wanted output format");

	let buffer_size = have.samples as usize * have.channels as usize;
	synth_context.init_buffer_queue(buffer_size, 3)?;
	synth_context.set_sample_rate(have.freq as f32);

	Ok(AudioCtx { device_id })
}

fn start_audio(audio_ctx: &mut AudioCtx) {
	unsafe { sdl::SDL_PauseAudioDevice(audio_ctx.device_id, 0); }
}
fn stop_audio(audio_ctx: &mut AudioCtx) {
	unsafe { sdl::SDL_PauseAudioDevice(audio_ctx.device_id, 1); }
}

unsafe extern fn audio_callback(ud: *mut std::os::raw::c_void, stream: *mut u8, length: i32) {
	use std::mem::transmute;

	let synth_context: &mut voi_synth::Context = transmute(ud);
	let buffer = synth_context.get_ready_buffer().expect("Failed to get ready buffer");

	buffer.copy_to(stream, length as usize);
	synth_context.queue_empty_buffer(buffer).unwrap();
}
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
mod lisp_frontend;
mod midi;

use voi_synth::*;
use window::*;
use lisp_frontend as lisp;
use std::time;

fn main() -> SynthResult<()> {
	std::env::set_var("RUST_BACKTRACE", "1");

	let _window = Window::new().expect("Window open failed");
	let mut synth_context = box voi_synth::Context::new(3, 256)?;

	// let midi_device = midi::init_device()?;

	// let voices = [
	// 	test_midi(&mut synth_context)?,
	// 	test_midi(&mut synth_context)?,
	// 	test_midi(&mut synth_context)?,
	// 	test_midi(&mut synth_context)?,

	// 	test_midi(&mut synth_context)?,
	// 	test_midi(&mut synth_context)?,
	// 	test_midi(&mut synth_context)?,
	// 	test_midi(&mut synth_context)?,
	// ];


	// let params = test_midi(&mut synth_context)?;
	// test_lisp(&mut synth_context)?;
	// test_sequencer(&mut synth_context)?;
	// test_feedback(&mut synth_context)?;
	// test_prebake(&mut synth_context)?;

	let voices = [
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,

		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,

		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,

		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
		test_perf(&mut synth_context)?,
	];

	let mut audio_device = init_audio(&mut synth_context).expect("Audio init failed");
	start_audio(&mut audio_device);

	let mut keys_down = [0; 8];

	for params in voices.iter() {
		if let Some(param) = params.get(1) {
			synth_context.set_parameter(*param, 0.0);
		}
	}

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

		// for message in midi_device.read() {
		// 	use midi::MidiMessage as Msg;

		// 	if let Msg::Packet([cmd, a, b]) = message {
		// 		println!("Packet {:02x} [{:02x}, {:02x}]", cmd, a, b);

		// 	} else {
		// 		println!("{:?}", message);
		// 	}

		// 	match message {
		// 		Msg::Control{controller, value, ..} => {
		// 			for params in voices.iter() {
		// 				if let Some(param) = params.get(controller as usize + 1) {
		// 					let value = value as f32 / 127.0;
		// 					synth_context.set_parameter(*param, value);
		// 				}
		// 			}
		// 		}

		// 		Msg::NoteOn{key, velocity, ..} => {
		// 			if let Some((k, voice)) = keys_down.iter_mut().zip(voices.iter()).find(|kv| *kv.0 == 0) {
		// 				*k = key;

		// 				if let Some(param) = voice.get(0) {
		// 					let key = key as f32;
		// 					let freq = 440.0 * 2.0f32.powf((key - 64.0) / 12.0);
		// 					synth_context.set_parameter(*param, freq);
		// 				}

		// 				let velocity = velocity as f32 / 127.0;
		// 				if let Some(param) = voice.get(1) {
		// 					synth_context.set_parameter(*param, velocity);
		// 				}
		// 			}
		// 		}

		// 		Msg::NoteOff{key, ..} => {
		// 			if let Some(pos) = keys_down.iter().position(|&k| k == key) {
		// 				keys_down[pos] = 0;

		// 				if let Some(param) = voices[pos].get(1) {
		// 					synth_context.set_parameter(*param, 0.0);
		// 				}	
		// 			}
		// 		}

		// 		_ => {}
		// 	}
		// }


		for _ in 0..4 {
			for v in voices.iter() {
				synth_context.set_parameter(v[0], 110.0);
			}
		}

		let begin = time::Instant::now();
		synth_context.dump_stats();
		let end = time::Instant::now();

		let diff = (end-begin).subsec_nanos() as f32 / 1000.0;

		println!("lock time {}us", diff);

		use std::thread::sleep;
		use std::time::Duration;
		sleep(Duration::from_millis(32));
	}

	stop_audio(&mut audio_device);

	Ok(())
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

	want.freq = 44100;
	// want.freq = 22050;
	want.format = AUDIO_F32SYS as u16;
	want.channels = 1;
	want.samples = 256;
	want.callback = Some(audio_callback);
	want.userdata = unsafe{ transmute(&mut **synth_context) };

	let device_id = unsafe {
		SDL_OpenAudioDevice(null(), 0, &want, &mut have, SDL_AUDIO_ALLOW_FREQUENCY_CHANGE as i32)
	};
	
	ensure!(device_id != 0, "Failed to open audio: {}", unsafe { from_cstr!(SDL_GetError()) } );
	ensure!(have.channels == 1, "Failed to get stereo audio");
	ensure!(have.format == AUDIO_F32SYS as _, "Failed to get wanted output format");

	let buffer_size = have.samples as usize * have.channels as usize;
	synth_context.set_buffer_size(buffer_size);
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
	// buffer.copy_to_stereo(stream, length as usize);
	synth_context.queue_empty_buffer(buffer).unwrap();
}



#[allow(dead_code)]
fn test_midi(synth_context: &mut voi_synth::Context) -> SynthResult<Vec<ParameterID>> {
	let mut synth = Synth::new();
	synth.set_gain(0.3);

	let feedback_store = synth.new_value_store();

	let velocity_param = synth.new_parameter();
	let freq_param = synth.new_parameter();
	let freq = synth.new_lowpass(freq_param, 10.0);

	let mod_param = synth.new_parameter();
	let mod_amt = synth.new_remap(mod_param, 0.0, 1.0,   0.0, 880.0);

	let feedback_param = synth.new_parameter();
	let feedback_amt = synth.new_remap(feedback_param, 0.0, 1.0,   0.0, 200.0);

	let feedback = synth.new_multiply(feedback_store, feedback_amt);
	let mod_freq = synth.new_add(freq, feedback);

	let mod_osc = synth.new_sine(mod_freq);
	let mod_a = synth.new_multiply(mod_osc, mod_amt);

	let osc0_freq = synth.new_add(freq, mod_a);
	let osc0 = synth.new_square(osc0_freq);

	let osc1_freq = synth.new_multiply(osc0_freq, 0.5);
	let osc1 = synth.new_saw(osc1_freq);

	let osc = synth.new_add(osc0, osc1);
	
	let env = synth.new_env_adsr(0.01, 0.1, 0.8, 1.0, velocity_param);
	let env = synth.new_power(env, 2.0);
	let out = synth.new_multiply(osc, env);

	let feedback = synth.new_sub(osc, feedback_store);
	synth.new_store_write(feedback_store, feedback);

	synth.set_output(out);
	synth.get_parameter(freq_param).set_value(440.0);

	synth_context.push_synth(synth)?;

	Ok(vec![
		freq_param,
		velocity_param,

		mod_param,
		feedback_param,
	])
}


#[allow(dead_code)]
fn test_perf(synth_context: &mut voi_synth::Context) -> SynthResult<Vec<ParameterID>> {
	let mut synth = Synth::new();
	synth.set_gain(0.3);

	let freq_param = synth.new_parameter();
	let freq = synth.new_lowpass(freq_param, 10.0);

	let mut osc = synth.new_sine(freq);

	for i in 0..3 {
		let freq = synth.new_multiply(freq, 1.0 + i as f32 / 30.0);
		let s = synth.new_sine(freq);
		osc = synth.new_add(osc, s);
	}

	synth.set_output(osc);
	synth.get_parameter(freq_param).set_value(440.0);

	synth_context.push_synth(synth)?;

	Ok(vec![freq_param])
}



#[allow(dead_code)]
fn test_lisp(synth_context: &mut voi_synth::Context) -> SynthResult<()> {
	use std::env;

	let default_script_path = "scripts/test0.voisynth";
	let script_path = env::args().skip(1)
		.next()
		.unwrap_or(default_script_path.into());

	println!("{:?}", script_path);

	let script = std::fs::read_to_string(script_path)?;
	lisp::evaluate(synth_context, &script)?;

	Ok(())
}



#[allow(dead_code)]
fn test_sequencer(synth_context: &mut voi_synth::Context) -> SynthResult<()> {
	let mut synth = Synth::new();
	synth.set_gain(0.3);

	let beat_rate = 180.0 / 60.0 * 2.0;

	let pulse = synth.new_square(beat_rate);
	let pulse2 = synth.new_square(beat_rate * 2.0);

	let pulse = synth.new_signal_to_control(pulse);
	let pulse2 = synth.new_signal_to_control(pulse2);
	let pulse = synth.new_multiply(pulse, pulse2); // pulse shortening

	let buf = synth.new_buffer(vec![55.0, 110.0, 220.0, 330.0, 220.0 * 5.0 / 4.0]);
	let seq = synth.new_sequencer(buf, pulse, 1.0);

	let osc = synth.new_sine(seq);
	let env = synth.new_env_ar(0.01, 0.7, pulse);
	let env = synth.new_power(env, 10.0);
	synth.new_multiply(osc, env);

	synth_context.push_synth(synth)?;

	Ok(())
}


#[allow(dead_code)]
fn test_feedback(synth_context: &mut voi_synth::Context) -> SynthResult<()> {
	{
		let mut synth = Synth::new();
		synth.set_gain(0.3);

		let mut feedback_chain = Vec::new();

		for _ in 0..32 {
			feedback_chain.push(synth.new_value_store());
		}

		let feedback_head = feedback_chain[0];
		let feedback_tail = feedback_chain[feedback_chain.len() - 1];

		// let mul_osc = synth.new_multiply(52.0, feedback_tail);
		// let mul_osc = synth.new_multiply(80.0, feedback_tail);
		let mul_osc = synth.new_multiply(110.0, feedback_tail);
		// let mul_osc = synth.new_multiply(220.0, feedback_tail);

		let fm = synth.new_saw(mul_osc);
		let fm = synth.new_multiply(fm, 180.0);

		let oscf0 = synth.new_add(220.0, fm);
		// let oscf1 = synth.new_multiply(oscf0, 0.51);
		// let oscf2 = synth.new_multiply(oscf0, 2.0);

		let osc = synth.new_sine(oscf0);
		// let osc2 = synth.new_square(oscf1);
		// let osc3 = synth.new_sine(oscf2);

		// let osc = synth.new_add(osc, osc2);
		// let osc = synth.new_add(osc, osc3);

		// let osc = synth.new_clamp(osc, -100.0, 1.0);
		let mul_osc = synth.new_sub(osc, feedback_tail);
		// let mul_osc = synth.new_sub(feedback_tail, osc);

		// let mul_osc_lfo = synth.new_square(2.0);
		// let mul_osc_lfo = synth.new_triangle(200.0);
		// let mul_osc_lfo = synth.new_signal_to_control(mul_osc_lfo);
		// let mul_osc_lfo = synth.new_power(mul_osc_lfo, 5.0);
		// let mul_osc_lfo = synth.new_control_to_signal(mul_osc_lfo);
		// let mul_osc = synth.new_multiply(mul_osc, mul_osc_lfo);

		for sd in feedback_chain.windows(2).rev() {
			if let &[src, dst] = sd {
				synth.new_store_write(dst, src);
			}
		}

		synth.new_store_write(feedback_head, mul_osc);
		synth.set_output(osc);

		synth_context.push_synth(synth)?;
	}

	let mut synth = Synth::new();
	synth.set_gain(200.0);

	let beat = synth.new_square(1.0);
	let env = synth.new_env_ar(0.08, 0.76, beat);

	let freq_mod = synth.new_multiply(env, 10.0);
	let freq = synth.new_add(25.0, freq_mod);
	let osc = synth.new_triangle(freq);

	let mixed = synth.new_multiply(osc, env);
	synth.set_output(mixed);

	// synth_context.push_synth(synth)?;

	Ok(())
}


#[allow(dead_code)]
fn test_prebake(synth_context: &mut voi_synth::Context) -> SynthResult<()> {
	let prebaked_buffer = {
		let mut synth = Synth::new();
		synth.set_gain(0.1);

		let osc_acc = synth.new_triangle(55.0);

		use std::cell::RefCell;
		let synth = RefCell::new(synth);

		let s_ref = || synth.borrow_mut();

		let osc = (0..10)
			.map(|i| s_ref().new_saw(110.0 + i as f32 / 10.0))
			.fold(osc_acc, |a, s| s_ref().new_add(a, s));

		let mut synth = synth.into_inner();

		synth.set_output(osc);

		let mut eval_ctx = context::EvaluationContext::new(44100.0);

		let mut buffer = Buffer::new(44100);
		synth.prewarm(44100, &mut eval_ctx);
		synth.evaluate_into_buffer(&mut buffer, &mut eval_ctx);
		buffer
	};

	let test_buffer = synth_context.create_shared_buffer(prebaked_buffer.data)?;

	let mut synth = Synth::new();
	synth.set_gain(1.0);

	let beat = synth.new_square(1.0);
	let env = synth.new_env_ar(0.08, 0.76, beat);

	let sampler = synth.new_sampler(test_buffer, 0.0);
	let mixed = synth.new_multiply(sampler, env);
	synth.set_output(mixed);

	synth_context.push_synth(synth)?;

	Ok(())
}
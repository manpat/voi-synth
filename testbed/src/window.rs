use std::mem;

use sdl::*;
use failure::Error;

#[allow(dead_code)]
pub struct Window {
	pub sdl_window: *mut SDL_Window,
}

pub struct EventIter;

impl Iterator for EventIter {
	type Item = (SDL_EventType, SDL_Event);

	fn next(&mut self) -> Option<Self::Item> {
		unsafe {
			let mut evt = mem::uninitialized();

			if SDL_PollEvent(&mut evt) != 0 {
				let evt_ty = mem::transmute(evt.type_);
				Some((evt_ty, evt))
			} else {
				None
			}
		}
	}
}

impl Window {
	pub fn new() -> Result<Self, Error> {
		unsafe {
			ensure!(SDL_Init(SDL_INIT_EVERYTHING) == 0, "SDL Init failed");

			let (window_width, window_height) = (200, 200);

			let sdl_window = SDL_CreateWindow(cstr!("voi-synth testbed"), 
				SDL_WINDOWPOS_UNDEFINED_MASK as i32, SDL_WINDOWPOS_UNDEFINED_MASK as i32, 
				window_width, window_height, 0);

			ensure!(!sdl_window.is_null(), "Window creation failed");

			Ok(Window { sdl_window })
		}
	}
}

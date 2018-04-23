

pub struct Buffer { pub data: Vec<f32> }

impl Buffer {
	pub fn new(buffer_size: usize) -> Buffer {
		Buffer{ data: vec![0.0; buffer_size] }
	}

	pub fn clear(&mut self) {
		for v in self.data.iter_mut() { *v = 0.0; }
	}

	pub unsafe fn copy_to(&self, dst: *mut u8, length: usize) {
		use std::mem::transmute;
		use std::ptr;

		let dst = transmute(dst);
		ptr::copy(self.data.as_ptr(), dst, self.data.len().min(length / 4));
	}
}

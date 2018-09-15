use std::thread;
use std::fs::{self, File};
use std::sync::mpsc::{self, Receiver};
use std::io::Read;
use std::io::BufReader;
use voi_synth::SynthResult;

#[derive(Debug, Copy, Clone)]
pub enum MidiMessage {
	NoteOff{ channel: u8, key: u8, velocity: u8 },
	NoteOn{ channel: u8, key: u8, velocity: u8 },
	Control{ channel: u8, controller: u8, value: u8 },
	PitchBend{ channel: u8, value: i16 },
	Packet([u8; 3])
}

pub struct MidiDevice (Receiver<MidiMessage>);

pub fn init_device() -> SynthResult<MidiDevice> {
	let entries = fs::read_dir("/dev")?;
	let midifile = entries
		.filter_map(|d| d.ok().as_ref().map(fs::DirEntry::path))
		.find(|p| {
			let filename = p.file_name().unwrap()
				.to_str().unwrap();

			!p.is_dir() && filename.starts_with("midi")
		});

	if let Some(midifile) = midifile {
		println!("Found midi device: {}", midifile.display());

		let mut reader = BufReader::new(File::open(midifile)?);
		let (tx, rx) = mpsc::channel();

		thread::spawn(move || {
			let mut packet = [0; 3];

			while reader.read_exact(&mut packet).is_ok() {
				tx.send(parse_midi_packet(packet)).unwrap();
			}
		});

		Ok(MidiDevice(rx))
	} else {
		bail!("Couldn't find midi device");
	}
}

impl MidiDevice {
	pub fn read(&self) -> mpsc::TryIter<MidiMessage> {
		self.0.try_iter()
	}
}

fn parse_midi_packet(packet: [u8; 3]) -> MidiMessage {
	if (packet[1] | packet[2]) & 0x80 != 0 {
		panic!("midi running status not supported!");
	}

	match packet[0] {
		cmd @ 0x80...0x8F => MidiMessage::NoteOff {
			channel: cmd & 0xF,
			key: packet[1],
			velocity: packet[2],
		},

		cmd @ 0x90...0x9F => {
			let channel = cmd & 0xF;
			let key = packet[1];
			let velocity = packet[2];

			if velocity == 0 {
				MidiMessage::NoteOff { channel, key, velocity }
			} else {
				MidiMessage::NoteOn { channel, key, velocity }
			}
		}

		cmd @ 0xB0...0xBF => MidiMessage::Control {
			channel: cmd & 0xF,
			controller: packet[1],
			value: packet[2],
		},

		cmd @ 0xE0...0xEF => {
			let channel = cmd & 0xF;
			let u_value = (packet[1] & 0x7F) as i16 | (packet[2] as i16 & 0x7F) << 7;
			let value = u_value - (1<<13);

			MidiMessage::PitchBend { channel, value }
		}

		_ => MidiMessage::Packet(packet),
	}
}
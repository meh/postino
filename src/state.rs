use std::io::{self, Write, Read};
use std::path::{Path, PathBuf};
use std::fs::File;
use json;
use mbox::Status;

pub struct State {
	path: PathBuf,
}

impl State {
	pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
		let     path = path.as_ref().to_path_buf();
		let mut file = try!(File::create(&path));
		try!(file.write_all(b"{}"));

		Ok(State {
			path: path,
		})
	}

	pub fn update<P: AsRef<Path>>(&mut self, path: P, status: Status) -> io::Result<()> {
		let mut state = {
			let mut file = try!(File::open(&self.path));
			let mut string = String::new();
			try!(file.read_to_string(&mut string));

			json::parse(&string).unwrap()
		};

		state[path.as_ref().to_string_lossy().as_ref()] = object!{
			"total"    => status.total,
			"seen"     => status.seen,
			"old"      => status.old,
			"answered" => status.answered,
			"flagged"  => status.flagged,
			"draft"    => status.draft,
			"deleted"  => status.deleted
		};

		{
			let mut file = try!(File::create(&self.path));
			try!(state.write_pretty(&mut file, 2));
		}

		Ok(())
	}
}

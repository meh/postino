//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//                    Version 2, December 2004
//
// Copyleft (ↄ) meh. <meh@schizofreni.co> | http://meh.schizofreni.co
//
// Everyone is permitted to copy and distribute verbatim or modified
// copies of this license document, and changing it is allowed as long
// as the name is changed.
//
//            DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
//   TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
//
//  0. You just DO WHAT THE FUCK YOU WANT TO.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use mailbox::{self, header};
use fs2::FileExt;

#[derive(Debug)]
pub struct MBox {
	path:       PathBuf,
	processing: AtomicBool,
}

#[derive(Eq, PartialEq, Copy, Clone, Default, Debug)]
pub struct Status {
	pub total:    usize,
	pub seen:     usize,
	pub old:      usize,
	pub answered: usize,
	pub flagged:  usize,
	pub draft:    usize,
	pub deleted:  usize,
}

impl MBox {
	pub fn open<P: AsRef<Path>>(path: P) -> io::Result<MBox> {
		Ok(MBox {
			path:       path.as_ref().to_path_buf(),
			processing: AtomicBool::new(false),
		})
	}

	/// Check if the `MBox` is currently being processed.
	pub fn is_processing(&self) -> bool {
		self.processing.load(Ordering::Relaxed)
	}

	/// Change the processing status.
	pub fn processing(&self, value: bool) {
		self.processing.store(value, Ordering::Relaxed);
	}

	/// Get the path.
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Process the status.
	pub fn status(&self) -> io::Result<Status> {
		let mut status = Status::default();
		let     input  = try!(File::open(&self.path));
		try!(input.lock_shared());

		for mail in mailbox::read(&input).body(false) {
			if let Ok(mail) = mail {
				let mut current = header::Status::empty();

				if let Some(Ok(s)) = mail.headers().get::<header::Status>() {
					current |= s;
				}

				if let Some(Ok(s)) = mail.headers().get_from::<header::Status, _>("X-Status") {
					current |= s;
				}

				status.total += 1;

				if current.contains(header::status::SEEN) {
					status.seen += 1;
				}

				if current.contains(header::status::OLD) {
					status.old += 1;
				}

				if current.contains(header::status::ANSWERED) {
					status.answered += 1;
				}

				if current.contains(header::status::FLAGGED) {
					status.flagged += 1;
				}

				if current.contains(header::status::DRAFT) {
					status.draft += 1;
				}

				if current.contains(header::status::DELETED) {
					status.deleted += 1;
				}
			}
		}

		try!(input.unlock());

		Ok(status)
	}
}

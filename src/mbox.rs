use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use mailbox::mail;
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

	pub fn is_processing(&self) -> bool {
		self.processing.load(Ordering::Relaxed)
	}

	pub fn processing(&self, value: bool) {
		self.processing.store(value, Ordering::Relaxed);
	}

	pub fn path(&self) -> &Path {
		&self.path
	}

	pub fn status(&self) -> io::Result<Status> {
		let mut status = Status::default();
		let     input  = try!(File::open(&self.path));
		try!(input.lock_shared());

		for mail in mail::read(&input).body(false) {
			if let Ok(mail) = mail {
				status.total += 1;

				for field in vec![mail.headers().get("Status"), mail.headers().get("X-Status")] {
					if let Some(Ok(mail::Header::Status(s))) = field {
						if s.contains(mail::status::SEEN) {
							status.seen += 1;
						}

						if s.contains(mail::status::OLD) {
							status.old += 1;
						}

						if s.contains(mail::status::ANSWERED) {
							status.answered += 1;
						}

						if s.contains(mail::status::FLAGGED) {
							status.flagged += 1;
						}

						if s.contains(mail::status::DRAFT) {
							status.draft += 1;
						}

						if s.contains(mail::status::DELETED) {
							status.deleted += 1;
						}
					}
				}
			}
		}

		try!(input.unlock());

		Ok(status)
	}
}

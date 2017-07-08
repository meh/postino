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

#![feature(mpsc_select)]

extern crate clap;
use clap::{Arg, App};

#[macro_use]
extern crate json;

extern crate threadpool;
use threadpool::ThreadPool;
extern crate num_cpus;

extern crate mailbox;
extern crate fs2;
extern crate notify;
use notify::{RecommendedWatcher, Watcher, DebouncedEvent, RecursiveMode};

use std::io;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::path::Path;
use std::time::Duration;

mod mbox;
use mbox::MBox;

mod state;
use state::State;

fn main() {
	let matches = App::new("postino")
		.version("0.1.0")
		.author("meh. <meh@schizofreni.co>")
		.about("Notify email status.")
		.arg(Arg::with_name("box")
			.short("b")
			.long("box")
			.takes_value(true)
			.multiple(true)
			.help("Path to a mail box to watch."))
		.arg(Arg::with_name("STATE")
			.index(1)
			.required(true)
			.help("The path to the state file."))
		.get_matches();

	// Create the state file.
	let mut state = State::open(matches.value_of("STATE").unwrap()).unwrap();

	// Create a threadpool to update the status for each box.
	let pool = ThreadPool::new(num_cpus::get());

	// Create a file system watcher.
	let (notify, notification) = channel();
	let mut watcher = RecommendedWatcher::new(notify, Duration::from_secs(0)).unwrap();

	// Create a map of mboxes.
	let (status, update) = channel();
	let mut boxes = HashMap::new();

	// For each box, watch changes to it and create a handler.
	for path in matches.values_of("box").unwrap() {
		let path = Path::new(path);

		if boxes.contains_key(path) {
			continue;
		}

		watcher.watch(path, RecursiveMode::NonRecursive).unwrap();
		boxes.insert(path.to_path_buf(), Arc::new(MBox::open(path).unwrap()));
	}

	// Pre-process all mboxes.
	for mbox in boxes.values() {
		process(mbox.clone(), &pool, status.clone());
	}

	loop {
		select! {
			// One of the mbox files has changed.
			event = notification.recv() => {
				match event.unwrap() {
					DebouncedEvent::Write(ref path) => {
						if let Some(mbox) = boxes.get(path) {
							process(mbox.clone(), &pool, status.clone());
						}
					}

					_ => ()
				}
			},

			// One of the mbox files has been processed.
			status = update.recv() => {
				let (mbox, status) = status.unwrap();
				mbox.processing(false);

				if let Ok(status) = status {
					state.update(mbox.path(), status).unwrap();
				}
			}
		}
	}
}

/// Process the `MBox` in the thread pool and send status to sender.
fn process(mbox: Arc<MBox>, pool: &ThreadPool, to: Sender<(Arc<MBox>, io::Result<mbox::Status>)>) {
	if !mbox.is_processing() {
		mbox.processing(true);

		// Process the mbox status in the thread pool.
		pool.execute(move || {
			to.send((mbox.clone(), mbox.status())).unwrap();
		});
	}
}

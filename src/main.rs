#![feature(mpsc_select)]

extern crate clap;
use clap::{Arg, App};

#[macro_use]
extern crate json;

extern crate futures;
use futures::{Future, Task};
extern crate futures_cpupool;
use futures_cpupool::{CpuPool};
extern crate num_cpus;

extern crate mailbox;
extern crate fs2;
extern crate notify;
use notify::{RecommendedWatcher, Watcher};

use std::io;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};
use std::path::Path;

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
	let pool = CpuPool::new(num_cpus::get() as u32);

	// Create a file system watcher.
	let (notify, notification) = channel();
	let mut watcher = RecommendedWatcher::new(notify).unwrap();

	// Create a map of mboxes.
	let (status, update) = channel();
	let mut boxes = HashMap::new();

	// For each box, watch changes to it and create a handler.
	for path in matches.values_of("box").unwrap() {
		let path = Path::new(path);

		if boxes.contains_key(path) {
			continue;
		}

		watcher.watch(path).unwrap();
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
				let event = event.unwrap();

				if event.path.is_none() || event.op.is_err() {
					continue;
				}

				if event.op.unwrap().contains(notify::op::WRITE) {
					if let Some(mbox) = boxes.get(event.path.as_ref().unwrap()) {
						process(mbox.clone(), &pool, status.clone());
					}
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

fn process(mbox: Arc<MBox>, pool: &CpuPool, to: Sender<(Arc<MBox>, io::Result<mbox::Status>)>) {
	if !mbox.is_processing() {
		mbox.processing(true);

		// Process the mbox status in the thread pool.
		Task::new().run(pool
			.execute(move || (mbox.clone(), mbox.status()))
			.then(move |res| to.send(res.unwrap()).map_err(|_| ())).boxed());
	}
}

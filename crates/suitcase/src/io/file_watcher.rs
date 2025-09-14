use notify::{Config, Event, RecommendedWatcher, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;

enum WatchCommand {
    ChangePath(PathBuf),
    Stop,
}

pub struct FileWatcher {
    pub event_rx: Receiver<Event>,
    command_tx: Sender<WatchCommand>,
    handle: Option<thread::JoinHandle<()>>,
}

impl FileWatcher {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
                move |res| {
                    if let Ok(event) = res {
                        event_tx.send(event).unwrap();
                    }
                },
                Config::default(),
            )
            .unwrap();

            let mut current_path: Option<PathBuf> = None;

            loop {
                if let Ok(cmd) = command_rx.recv_timeout(Duration::from_millis(100)) {
                    match cmd {
                        WatchCommand::ChangePath(new_path) => {
                            if let Some(path) = &current_path {
                                watcher.unwatch(path).unwrap();
                            }
                            watcher
                                .watch(&new_path, notify::RecursiveMode::Recursive)
                                .unwrap();
                            current_path = Some(new_path);
                        }
                        WatchCommand::Stop => {
                            if let Some(path) = &current_path {
                                watcher.unwatch(path).unwrap();
                            }
                            break;
                        }
                    }
                }
            }
        });

        Self {
            event_rx,
            command_tx,
            handle: Some(handle),
        }
    }

    pub fn change_path(&self, new_path: &PathBuf) {
        let _ = self
            .command_tx
            .send(WatchCommand::ChangePath(new_path.clone()));
    }

    pub fn stop(mut self) {
        let _ = self.command_tx.send(WatchCommand::Stop);
        if let Some(handle) = self.handle.take() {
            handle.join().expect("Failed to join file watcher thread");
        }
    }
}

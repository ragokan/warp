use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use std::{io, thread};

use instant::Instant;
use mio::Interest;
use parking_lot::FairMutex;

use super::event_loop::{EventLoop, PTY_TOKEN, SIGNALS_TOKEN};
use super::*;
use crate::terminal::event_listener::ChannelEventListener;
use crate::terminal::writeable_pty::Message;
use crate::terminal::{SizeInfo, TerminalModel};

struct TestPty {
    reader: TestReader,
    writer: io::Sink,
    killed: Arc<AtomicBool>,
}

enum TestReader {
    Empty,
    #[cfg(unix)]
    Pipe(mio::unix::pipe::Receiver, Arc<AtomicUsize>),
}

impl io::Read for TestReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Empty => Ok(0),
            #[cfg(unix)]
            Self::Pipe(receiver, reads) => {
                reads.fetch_add(1, Ordering::SeqCst);
                receiver.read(buf)
            }
        }
    }
}

impl EventedReadWrite for TestPty {
    type Reader = TestReader;
    type Writer = io::Sink;

    fn register(&mut self, poll: &mio::Poll, _: Interest) -> io::Result<()> {
        #[cfg(unix)]
        if let TestReader::Pipe(receiver, _) = &mut self.reader {
            poll.registry()
                .register(receiver, PTY_TOKEN, Interest::READABLE)?;
        }
        Ok(())
    }

    fn reregister(&mut self, _: &mio::Poll, _: Interest) -> io::Result<()> {
        Ok(())
    }

    fn deregister(&mut self, _: &mio::Poll) -> io::Result<()> {
        Ok(())
    }

    fn reader(&mut self) -> &mut Self::Reader {
        &mut self.reader
    }

    fn writer(&mut self) -> &mut Self::Writer {
        &mut self.writer
    }

    fn read_token(&self) -> mio::Token {
        PTY_TOKEN
    }

    fn write_token(&self) -> mio::Token {
        PTY_TOKEN
    }
}

impl EventedPty for TestPty {
    fn child_event_token(&self) -> mio::Token {
        SIGNALS_TOKEN
    }

    fn next_child_event(&mut self) -> Option<ChildEvent> {
        None
    }

    fn on_resize(&mut self, _: &SizeInfo) {}

    fn kill(self) -> anyhow::Result<()> {
        self.killed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

fn stop_loop(
    terminal: Arc<FairMutex<TerminalModel>>,
    sender: &mio_channel::Sender<Message>,
    receiver: mio_channel::Receiver<Message>,
    message: Message,
) -> (mio_channel::Receiver<Message>, bool) {
    let killed = Arc::new(AtomicBool::new(false));
    let pty = TestPty {
        reader: TestReader::Empty,
        writer: io::sink(),
        killed: killed.clone(),
    };
    let handle = EventLoop::new(
        terminal,
        ChannelEventListener::new_for_test(),
        pty,
        receiver,
    )
    .spawn();
    sender.send(message).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "event loop did not stop");
        thread::sleep(Duration::from_millis(1));
    }
    (handle.join().unwrap(), killed.load(Ordering::SeqCst))
}

#[cfg(unix)]
#[test]
fn pty_read_waits_for_terminal_lock_without_spinning() {
    use std::io::Write as _;

    let terminal = Arc::new(FairMutex::new(TerminalModel::mock(None, None)));
    let guard = terminal.lock();
    let (mut writer, reader) = mio::unix::pipe::new().unwrap();
    let reads = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = mio_channel::channel();
    let pty = TestPty {
        reader: TestReader::Pipe(reader, reads.clone()),
        writer: io::sink(),
        killed: Arc::new(AtomicBool::new(false)),
    };
    let handle = EventLoop::new(
        terminal.clone(),
        ChannelEventListener::new_for_test(),
        pty,
        receiver,
    )
    .spawn();
    writer.write_all(b"0123456789").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while reads.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "PTY was never read");
        thread::sleep(Duration::from_millis(1));
    }
    thread::sleep(Duration::from_millis(200));
    let count = reads.load(Ordering::SeqCst);
    drop(guard);
    sender.send(Message::Shutdown).unwrap();
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "event loop did not stop");
        thread::sleep(Duration::from_millis(1));
    }
    handle.join().unwrap();
    assert!(count < 100, "read {count} times while terminal was locked");
}

#[test]
fn planned_replacement_preserves_terminal_and_reuses_channel() {
    let terminal = Arc::new(FairMutex::new(TerminalModel::mock(None, None)));
    let (sender, receiver) = mio_channel::channel();
    let (receiver, killed) = stop_loop(
        terminal.clone(),
        &sender,
        receiver,
        Message::ShutdownForReplacement,
    );
    assert!(killed);
    assert!(!terminal.lock().is_read_only());
    let (_, killed) = stop_loop(
        terminal.clone(),
        &sender,
        receiver,
        Message::ShutdownForReplacement,
    );
    assert!(killed);
    assert!(!terminal.lock().is_read_only());
}

#[test]
fn normal_shutdown_still_exits_terminal() {
    let terminal = Arc::new(FairMutex::new(TerminalModel::mock(None, None)));
    let (sender, receiver) = mio_channel::channel();
    let (_, killed) = stop_loop(terminal.clone(), &sender, receiver, Message::Shutdown);
    assert!(killed);
    assert!(terminal.lock().is_read_only());
}

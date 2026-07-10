use crossbeam_channel::{Receiver, Sender, unbounded};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    StartRecording,
    StopRecording,
    StartStreaming,
    StopStreaming,
    AddSource { name: String },
    RemoveSource { name: String },
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    pub fps: f32,
    pub frame_time_ms: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    RecordingStarted,
    RecordingStopped,
    StreamingStarted,
    StreamingStopped,
    FrameStats(FrameStats),
    Error(String),
}

/// Owns both ends for now; later phases can move the core ends to worker threads.
#[derive(Debug)]
pub struct CoreChannels {
    pub command_tx: Sender<Command>,
    pub command_rx: Receiver<Command>,
    pub event_tx: Sender<Event>,
    pub event_rx: Receiver<Event>,
}

pub fn core_channels() -> CoreChannels {
    let (command_tx, command_rx) = unbounded();
    let (event_tx, event_rx) = unbounded();

    CoreChannels {
        command_tx,
        command_rx,
        event_tx,
        event_rx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_and_events_cross_their_channels() {
        let channels = core_channels();

        channels.command_tx.send(Command::StartRecording).unwrap();
        assert_eq!(channels.command_rx.try_recv(), Ok(Command::StartRecording));

        channels.event_tx.send(Event::RecordingStarted).unwrap();
        assert_eq!(channels.event_rx.try_recv(), Ok(Event::RecordingStarted));
    }
}

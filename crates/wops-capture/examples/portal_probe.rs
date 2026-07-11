use std::time::{Duration, Instant};

use wops_capture::{CaptureEvent, VideoSource, capture_channels, portal::PortalCapture};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channels = capture_channels();
    let mut capture = PortalCapture::screen(None);
    capture.start(channels.frame_sink, channels.event_sink)?;

    let started = Instant::now();
    let mut frames = 0_u64;
    while started.elapsed() < Duration::from_secs(30) {
        for event in channels.event_rx.try_iter() {
            match event {
                CaptureEvent::RestoreToken(_) => println!("restore token received"),
                other => println!("event: {other:?}"),
            }
        }
        while channels.frame_rx.try_recv().is_ok() {
            frames += 1;
        }
        if frames > 0 {
            println!("received {frames} frames");
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    capture.stop();
    if frames == 0 {
        return Err("portal probe did not receive a frame before timeout".into());
    }
    Ok(())
}

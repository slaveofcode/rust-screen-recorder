use screenshots::{Screen, display_info::DisplayInfo};
use std::{time::Instant, io, thread};

use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use webm::mux;
use webm::mux::Track;

#[derive(Debug, serde::Deserialize)]
struct Args {
    arg_path: PathBuf,
    flag_codec: Codec,
    flag_time: Option<u64>,
    flag_fps: u64,
    flag_quality: Quality,
}

#[derive(Debug, serde::Deserialize)]
enum Quality {
    Best,
    Balanced,
    Low,
}

#[derive(Debug, serde::Deserialize)]
enum Codec {
    Vp8,
    Vp9,
}

mod encoder;

fn main() {
    let start = Instant::now();

    // let screens = Screen::all().expect("error");

    // println!("{:?}", screens.len());
 
    // for screen in screens {
    //     println!("capturer {screen:?}");
    //     let mut image = screen.capture().unwrap();
    //     image
    //         .save(format!("target/{}.png", screen.display_info.id))
    //         .unwrap();

    //     // image = screen.capture_area(300, 300, 300, 300).unwrap();
    //     // image
    //     //    .save(format!("target/{}-2.png", screen.display_info.id))
    //     //    .unwrap();
    // }

    let dis = DisplayInfo::all().unwrap()[0];
    let screen = Screen::new(&dis);
    //let screen = Screen::from_point(100, 100).unwrap();
    println!("capturer {screen:?}");

    //let image = screen.capture_area(300, 300, 300, 300).unwrap();
    let image = screen.capture().unwrap();
    image.save("captures/capture_display_with_point.png").unwrap();
    println!("time: {:?}", start.elapsed());
}

fn record_screen(duration: Duration) -> io::Result<()> {
    // let duration = Some(Duration::from_millis(5000));
    let out = match {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open("./captures/macos_rec.mp4")
    } {
        Ok(file) => file,
        Err(ref e) if e.kind() == io::ErrorKind::AlreadyExists => {
            if loop {
                quest::ask("Overwrite the existing file? [y/N] ");
                if let Some(b) = quest::yesno(false)? {
                    break b;
                }
            } {
                File::create("./captures/macos_rec.mp4")?
            } else {
                return Ok(());
            }
        }
        Err(e) => return Err(e.into()),
    };
    
    let mut webm =
        mux::Segment::new(mux::Writer::new(out)).expect("Could not initialize the multiplexer.");
    
    let codec = Codec::Vp9;

    let (vpx_codec, mux_codec) = match codec {
        Codec::Vp8 => (encoder::VpxVideoCodecId::VP8, mux::VideoCodecId::VP8),
        Codec::Vp9 => (encoder::VpxVideoCodecId::VP9, mux::VideoCodecId::VP9),
    };

    let mut video_track = webm.add_video_track(width, height, None, mux_codec);

    let quality = Quality::Balanced;
    // Setup the encoder.
    let quality = match quality {
        Quality::Best => Q::Best,
        Quality::Balanced => Q::Balanced,
        Quality::Low => Q::Low,
    };
    let mut vpx = vpx_encode::VpxEncoder::new(EncoderCfg::VPX(vpx_encode::VpxEncoderConfig {
        width,
        height,
        quality,
        codec: vpx_codec,
        keyframe_interval: None,
    }))
    .unwrap();

    // Start recording.

    let start = Instant::now();
    let stop = Arc::new(AtomicBool::new(false));

    thread::spawn({
        let stop = stop.clone();
        move || {
            let _ = quest::ask("Recording! Press ⏎ to stop.");
            let _ = quest::text();
            stop.store(true, Ordering::Release);
        }
    });

    let fps = 30;

    let spf = Duration::from_nanos(1_000_000_000 / fps);

    // Capturer object is expensive, avoiding to create it frequently.
    let mut c = Capturer::new(d, true).unwrap();
    while !stop.load(Ordering::Acquire) {
        let now = Instant::now();
        let time = now - start;

        if Some(true) == duration.map(|d| time > d) {
            break;
        }

        if let Ok(frame) = c.frame(Duration::from_millis(0)) {
            let ms = time.as_secs() * 1000 + time.subsec_millis() as u64;

            for frame in vpx.encode(ms as i64, &frame, STRIDE_ALIGN).unwrap() {
                video_track.add_frame(frame.data, frame.pts as u64 * 1_000_000, frame.key);
            }
        }

        let dt = now.elapsed();
        if dt < spf {
            thread::sleep(spf - dt);
        }
    }

    // End things.

    let _ = webm.finalize(None);

    Ok(())
}

extern crate gst;

use std::sync::{Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub fn stream(width: usize, height: usize, fps: usize, rtmp_uri: &str) {
    gst::init();
    let pipeline_str = format!(
        concat!(
            "appsrc caps=\"video/x-raw,format=RGB,width={},height={},framerate={}/1\" name=appsrc0 ! ",
            "videoconvert ! video/x-raw, format=I420, width={}, height={}, framerate={}/1 ! ",
            "queue ! x264enc ! h264parse ! ",
            "flvmux streamable=true name=mux ! ",
            "rtmpsink location={} ",
            "audiotestsrc ! voaacenc bitrate=128000 ! mux."
        ),
        width, height, fps,
        width, height, fps,
        rtmp_uri
    );
    let mut pipeline = gst::Pipeline::new_from_str(&pipeline_str).unwrap();
    let mut mainloop = gst::MainLoop::new();
    let appsrc = pipeline
        .get_by_name("appsrc0")
        .expect("Couldn't get appsrc from pipeline");
    let mut appsrc = gst::AppSrc::new_from_element(appsrc);
    let mut bufferpool = gst::BufferPool::new().unwrap();
    let appsrc_caps = appsrc.caps().unwrap();
    bufferpool.set_params(&appsrc_caps, (width * height * 3) as _, 0, 0);
    if bufferpool.set_active(true).is_err() {
        panic!("Couldn't activate buffer pool");
    }
    mainloop.spawn();
    pipeline.play();

    thread::spawn(move || {
        let condvar = Condvar::new();
        let mutex = Mutex::new(());
        let mut gray = 0;
        loop {
            if let Some(mut buffer) = bufferpool.acquire_buffer() {
                buffer
                    .map_write(|mapping| {
                        for c in mapping.iter_mut::<u8>() {
                            *c = gray;
                        }
                    })
                    .ok();
                gray += 1;
                gray %= 255;
                appsrc.push_buffer(buffer);
                let guard = mutex.lock().unwrap();
                condvar
                    .wait_timeout(guard, Duration::from_millis((1000 / fps) as _))
                    .ok();
            } else {
                println!("Couldn't get buffer, sending EOS and finishing thread");
                appsrc.end_of_stream();
                break;
            }
        }
    });

    #[cfg(feature = "logging")]
    for message in pipeline.bus().unwrap().receiver().iter() {
        match message.parse() {
            gst::Message::StateChangedParsed {
                ref old, ref new, ..
            } => {
                println!(
                    "element `{}` changed from {:?} to {:?}",
                    message.src_name(),
                    old,
                    new
                );
            }
            gst::Message::ErrorParsed {
                ref error,
                ref debug,
                ..
            } => {
                println!(
                    "error msg from element `{}`: {}, {}. Quitting",
                    message.src_name(),
                    error.message(),
                    debug
                );
                break;
            }
            gst::Message::Eos(_) => {
                println!("eos received quiting");
                break;
            }
            _ => {
                println!(
                    "msg of type `{}` from element `{}`",
                    message.type_name(),
                    message.src_name()
                );
            }
        }
    }
    mainloop.quit();
}

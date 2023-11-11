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
    bufferpool.set_params(&appsrc_caps, (width * height * 3) as _, 3, 3);
    if bufferpool.set_active(true).is_err() {
        panic!("Couldn't activate buffer pool");
    }
    mainloop.spawn();
    pipeline.play();

    thread::spawn(move || {
        let mut gray = 0;
        loop {
            let start = std::time::Instant::now();
            if let Some(mut buffer) = bufferpool.acquire_buffer() {
                println!("Time acquiring: {}ms", start.elapsed().as_millis());
                let start = std::time::Instant::now();
                buffer
                    .map_write(|mapping| {
                        for (y, row) in mapping.data_mut::<u8>().chunks_exact_mut(width * 3).enumerate() {
                            for (x, rgb) in row.chunks_exact_mut(3).enumerate() {
                                if let [r, g, b] = rgb {
                                    *r = gray;
                                    *g = (x * 255 / width) as _;
                                    *b = (y * 255 / height) as _;
                                }
                            }
                        }
                    })
                    .ok();
                println!("Time drawing: {}ms", start.elapsed().as_millis());
                gray += 1;
                gray %= 255;
                let start = std::time::Instant::now();
                appsrc.push_buffer(buffer);
                println!("Time pushing: {}ms", start.elapsed().as_millis());
            } else {
                println!("Couldn't get buffer, sending EOS and finishing thread");
                appsrc.end_of_stream();
                break;
            }
        }
    });

    for _message in pipeline.bus().unwrap().receiver().iter() {
        // match message.parse() {
        //     gst::Message::StateChangedParsed {
        //         ref old, ref new, ..
        //     } => {
        //         println!(
        //             "element `{}` changed from {:?} to {:?}",
        //             message.src_name(),
        //             old,
        //             new
        //         );
        //     }
        //     gst::Message::ErrorParsed {
        //         ref error,
        //         ref debug,
        //         ..
        //     } => {
        //         println!(
        //             "error msg from element `{}`: {}, {}. Quitting",
        //             message.src_name(),
        //             error.message(),
        //             debug
        //         );
        //         break;
        //     }
        //     gst::Message::Eos(_) => {
        //         println!("eos received quiting");
        //         break;
        //     }
        //     _ => {
        //         println!(
        //             "msg of type `{}` from element `{}`",
        //             message.type_name(),
        //             message.src_name()
        //         );
        //     }
        // }
    }
    mainloop.quit();
}

extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate gstreamer_video as gst_video;

use crate::renderer::*;
use gst::{prelude::*, Caps, ElementFactory};

pub fn stream(
    size: (usize, usize),
    fps: usize,
    audio_bitrate: usize,
    rtmp_uri: &str,
    mut draw_frame: impl FnMut(&mut Frame) + Send + Sync + 'static,
) {
    // let pipeline_str = format!(
    //     concat!(
    //         "appsrc caps=\"video/x-raw,format=RGB,width={},height={},framerate={}/1\" name=appsrc0 ! ",
    //         "v4l2convert ! video/x-raw, format=I420, width={}, height={}, framerate={}/1 ! ",
    //         "x264enc ! h264parse ! ",
    //         "flvmux streamable=true name=mux ! ",
    //         "rtmpsink location={} ",
    //         "audiotestsrc ! voaacenc bitrate=128000 ! mux."
    //     ),
    //     width, height, fps,
    //     width, height, fps,
    //     rtmp_uri
    // );

    gst::init().unwrap();
    let pipeline = gst::Pipeline::default();

    // * Source
    let (width, height) = size;
    let video_info =
        gst_video::VideoInfo::builder(gst_video::VideoFormat::Rgb, width as u32, height as u32)
            .fps(gst::Fraction::new(fps as _, 1))
            .build()
            .unwrap();
    let video_source = gst_app::AppSrc::builder()
        .caps(&video_info.to_caps().unwrap())
        .is_live(true)
        .format(gst::Format::Time)
        .build();

    // * Convert
    let videoconvert = ElementFactory::make("v4l2convert").build().unwrap();
    let caps_filter = ElementFactory::make("capsfilter")
        .property(
            "caps",
            Caps::builder("video/x-raw").field("format", "I420").build(),
        )
        .build()
        .unwrap();
    let video_encoder = ElementFactory::make("x264enc").build().unwrap();
    let video_decoder = ElementFactory::make("h264parse").build().unwrap();

    // * Mux
    let mux = ElementFactory::make("flvmux")
        .property("streamable", true)
        .build()
        .unwrap();

    // * Sink
    let rtmp_sink = ElementFactory::make("rtmpsink")
        .property("location", rtmp_uri)
        .build()
        .unwrap();

    // * Audio
    let audio_source = ElementFactory::make("audiotestsrc").build().unwrap();
    let audio_encoder = ElementFactory::make("voaacenc")
        .property("bitrate", audio_bitrate as i32)
        .build()
        .unwrap();

    // * Add
    pipeline
        .add_many([
            video_source.upcast_ref(),
            &videoconvert,
            &caps_filter,
            &video_encoder,
            &video_decoder,
            &mux,
            &rtmp_sink,
            &audio_source,
            &audio_encoder,
        ])
        .unwrap();

    // * Link video
    gst::Element::link_many([
        video_source.upcast_ref(),
        &videoconvert,
        &caps_filter,
        &video_encoder,
        &video_decoder,
        &mux,
        &rtmp_sink,
    ])
    .unwrap();

    // * Link audio
    gst::Element::link_many([&audio_source, &audio_encoder, &mux]).unwrap();

    // * Draw callback
    let hungry_need = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let hungry_enough = hungry_need.clone();
    let hungry_client = hungry_need.clone();
    video_source.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(move |_, _| {
                *hungry_need.0.lock().unwrap() = true;
                hungry_need.1.notify_one();
            })
            .enough_data(move |_| {
                *hungry_enough.0.lock().unwrap() = false;
                hungry_enough.1.notify_one();
            })
            .build(),
    );
    std::thread::spawn(move || loop {
        let mut started = hungry_client.0.lock().unwrap();
        while !*started {
            started = hungry_client.1.wait(started).unwrap();
        }

        let mut buffer = gst::Buffer::with_size(video_info.size()).unwrap();
        {
            std::thread::sleep(std::time::Duration::from_millis(33));
            let mut buffer = buffer.get_mut().unwrap().map_writable().unwrap();
            let mut frame = crate::renderer::Frame::new(buffer.as_mut_slice(), width, height);

            draw_frame(&mut frame);
        };

        video_source.push_buffer(buffer).unwrap();
    });

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in pipeline.bus().unwrap().iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                panic!("{}", err);
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();
}

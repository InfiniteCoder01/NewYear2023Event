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
    let video_source = ElementFactory::make("videotestsrc").build().unwrap();
    // gst_app::AppSrc::builder()
    //     .caps(&video_info.to_caps().unwrap())
    //     .is_live(true)
    //     .format(gst::Format::Time)
    //     .build();

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
    // video_source.set_callbacks(
    //     gst_app::AppSrcCallbacks::builder()
    //         .need_data(move |appsrc, _| {
    //             let mut buffer = gst::Buffer::with_size(video_info.size()).unwrap();
    //             {
    //                 let mut buffer = buffer.get_mut().unwrap().map_writable().unwrap();
    //                 let mut frame =
    //                     crate::renderer::Frame::new(buffer.as_mut_slice(), width, height);

    //                 draw_frame(&mut frame);
    //             };

    //             appsrc.push_buffer(buffer).unwrap();
    //         })
    //         .build(),
    // );

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

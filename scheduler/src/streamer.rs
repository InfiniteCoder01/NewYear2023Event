pub extern crate gstreamer as gst;
pub extern crate gstreamer_audio as gst_audio;
pub extern crate gstreamer_base as gst_base;
pub extern crate gstreamer_video as gst_video;

use gst::{prelude::*, Caps, ElementFactory};

pub fn stream<F>(
    size: (usize, usize),
    video_bitrate: usize,
    _audio_samplerate: usize,
    audio_bitrate: usize,
    rtmp_uri: &str,
    draw_frame: F,
    virtual_mode: bool,
) where
    F: FnMut(cairo::Context, f64, f64) + Send + Sync + 'static,
{
    // let pipeline_str = format!(
    //     concat!(
    //         "videotestsrc pattern=black ! cairooverlay ! ",
    //         "videoconvert ! video/x-raw, format=I420, width={}, height={}, framerate={}/1 ! ",
    //         "x264enc ! h264parse ! ",
    //         "flvmux streamable=true name=mux ! ",
    //         "rtmp2sink location={} ",
    //         "pulsesrc ! ",
    //         "voaacenc bitrate=128000 ! mux."
    //     ),
    //     width, height, fps,
    //     width, height, fps,
    //     rtmp_uri
    // );

    gst::init().unwrap();
    let pipeline = gst::Pipeline::default();

    let (enc, parse, cvt) = ("x264enc", "h264parse", "v4l2convert");
    // let (enc, parse, cvt) = ("x264enc", "h264parse", "videoconvert");

    // * Source
    let (width, height) = size;
    let background = ElementFactory::make("videotestsrc")
        .property_from_str("pattern", "black")
        .build()
        .unwrap();
    let video_overlay = ElementFactory::make("cairooverlay").build().unwrap();
    let source_caps_filter = ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst_video::VideoCapsBuilder::new()
                .width(width as _)
                .height(height as _)
                .build(),
        )
        .build()
        .unwrap();

    // * Convert
    let videoconvert = ElementFactory::make(cvt).build().unwrap();
    let youtube_caps_filter = ElementFactory::make("capsfilter")
        .property(
            "caps",
            Caps::builder("video/x-raw").field("format", "I420").build(),
        )
        .build()
        .unwrap();
    let video_encoder = ElementFactory::make(enc)
        .property("key-int-max", 30_u32)
        .property("bitrate", video_bitrate as u32)
        .property_from_str("speed-preset", "ultrafast")
        .build()
        .unwrap();
    let video_decoder = ElementFactory::make(parse).build().unwrap();

    // * Mux
    let mux = ElementFactory::make("flvmux")
        .property("streamable", true)
        .build()
        .unwrap();

    // * Sink
    let rtmp_sink = ElementFactory::make("rtmp2sink")
        .property("location", rtmp_uri)
        .build()
        .unwrap();

    // * Audio
    let audio_source = ElementFactory::make("pulsesrc").build().unwrap();
    let audio_encoder = ElementFactory::make("voaacenc")
        .property("bitrate", audio_bitrate as i32)
        .build()
        .unwrap();

    // * Virtual mode
    let basic_video_sink = ElementFactory::make("autovideosink").build().unwrap();
    let basic_audio_sink = ElementFactory::make("autoaudiosink").build().unwrap();

    if virtual_mode {
        // * Add elements
        pipeline
            .add_many([
                &background,
                &video_overlay,
                &source_caps_filter,
                &videoconvert,
                &basic_video_sink,
                &audio_source,
                &basic_audio_sink,
            ])
            .unwrap();

        // * Link video
        gst::Element::link_many([
            &background,
            &video_overlay,
            &source_caps_filter,
            &videoconvert,
            &basic_video_sink,
        ])
        .unwrap();

        // * Link audio
        gst::Element::link_many([&audio_source, &basic_audio_sink]).unwrap();
    } else {
        // * Add elements
        pipeline
            .add_many([
                &background,
                &video_overlay,
                &source_caps_filter,
                &videoconvert,
                &youtube_caps_filter,
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
            &background,
            &video_overlay,
            &source_caps_filter,
            &videoconvert,
            &youtube_caps_filter,
            &video_encoder,
            &video_decoder,
            &mux,
            &rtmp_sink,
        ])
        .unwrap();

        // * Link audio
        gst::Element::link_many([&audio_source, &audio_encoder, &mux]).unwrap();
    }

    // * Draw callback
    let draw_frame = std::sync::Mutex::new(draw_frame);
    video_overlay.connect("draw", false, move |args| {
        draw_frame.lock().unwrap()(
            args[1].get::<cairo::Context>().unwrap(),
            width as _,
            height as _,
        );
        None
    });

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in pipeline.bus().unwrap().iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                panic!(
                    "Element {}:\n{}",
                    err.src().map_or(String::from("None"), |element| element
                        .name()
                        .as_str()
                        .to_owned()),
                    err
                );
            }
            MessageView::Warning(warning) => {
                eprintln!(
                    "Warning from element {}:\n{}",
                    warning.src().map_or(String::from("None"), |element| element
                        .name()
                        .as_str()
                        .to_owned()),
                    warning
                );
            }
            MessageView::Info(info) => {
                eprintln!(
                    "Info from element {}:\n{}",
                    info.src().map_or(String::from("None"), |element| element
                        .name()
                        .as_str()
                        .to_owned()),
                    info
                );
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();
}

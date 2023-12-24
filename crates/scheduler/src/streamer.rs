pub extern crate gstreamer as gst;
pub extern crate gstreamer_audio as gst_audio;
pub extern crate gstreamer_base as gst_base;
pub extern crate gstreamer_video as gst_video;

use gst::{prelude::*, Caps, ElementFactory};

pub fn stream<F>(
    size: (usize, usize),
    video_bitrate: usize,
    audio_bitrate: usize,
    rtmp_uri: &str,
    draw_frame: F,
    virtual_mode: bool,
) where
    F: FnMut(cairo::Context, f64, f64) + Send + Sync + 'static,
{
    // let pipeline_str = format!(
    //     concat!(
    //         "videotestsrc pattern=black ! cairooverlay ! width={}, height={}, format=BGRx ! ",
    //         "videoconvert ! video/x-raw, format=I420 ! ",
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

    let (enc, parse, cvt, audioenc) = if virtual_mode {
        ("x264enc", "h264parse", "videoconvert", "faac")
    } else {
        ("x264enc", "h264parse", "v4l2convert", "voaacenc")
    };

    // * Source
    let (width, height) = size;
    let background = ElementFactory::make("videotestsrc")
        .property_from_str("pattern", "black")
        .build()
        .unwrap();
    let background_caps_filter = ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst_video::VideoCapsBuilder::new()
                .width(width as _)
                .height(height as _)
                .format(gst_video::VideoFormat::Bgrx)
                .build(),
        )
        .build()
        .unwrap();
    let video_overlay = ElementFactory::make("cairooverlay").build().unwrap();
    let channel_swap_fixer = ElementFactory::make("rawvideoparse")
        .property("use-sink-caps", false)
        .property("width", width as i32)
        .property("height", height as i32)
        .property("format", gst_video::VideoFormat::Rgbx)
        .build()
        .unwrap();
    let source_caps_filter = ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst_video::VideoCapsBuilder::new()
                .width(width as _)
                .height(height as _)
                .format(gst_video::VideoFormat::Bgrx)
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
    let audio_encoder = ElementFactory::make(audioenc)
        .property("bitrate", audio_bitrate as i32)
        .build()
        .unwrap();

    // * Virtual mode
    let basic_video_sink = ElementFactory::make("autovideosink").build().unwrap();

    if virtual_mode {
        // * Add elements
        pipeline
            .add_many([
                &background,
                &background_caps_filter,
                &video_overlay,
                &source_caps_filter,
                &videoconvert,
                &basic_video_sink,
            ])
            .unwrap();

        // * Link video
        gst::Element::link_many([
            &background,
            &background_caps_filter,
            &video_overlay,
            &source_caps_filter,
            &videoconvert,
            &basic_video_sink,
        ])
        .unwrap();
    } else {
        // * Add elements
        pipeline
            .add_many([
                &background,
                &background_caps_filter,
                &video_overlay,
                &channel_swap_fixer,
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
            &background_caps_filter,
            &video_overlay,
            &channel_swap_fixer,
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

    let result = pipeline.set_state(gst::State::Playing);

    for msg in pipeline.bus().unwrap().iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                log::error!(
                    "Element {}: {}",
                    err.src().map_or(String::from("None"), |element| element
                        .name()
                        .as_str()
                        .to_owned()),
                    err
                );
            }
            MessageView::Warning(warning) => {
                log::warn!(
                    "Element {}: {}",
                    warning.src().map_or(String::from("None"), |element| element
                        .name()
                        .as_str()
                        .to_owned()),
                    warning
                );
            }
            MessageView::Info(info) => {
                log::info!(
                    "Element {}: {}",
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

    result.unwrap();

    pipeline.set_state(gst::State::Null).unwrap();
}

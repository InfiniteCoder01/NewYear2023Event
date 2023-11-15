extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate gstreamer_video as gst_video;

use gst::{prelude::*, Caps, ElementFactory};

pub fn stream<F>(
    size: (usize, usize),
    fps: usize,
    audio_bitrate: usize,
    rtmp_uri: &str,
    mut draw_frame: F,
) where
    F: FnMut(cairo::Context, f64, f64) + Send + Sync + 'static,
{
    // let pipeline_str = format!(
    //     concat!(
    //         "cairooverlay ! ",
    //         "videoconvert ! video/x-raw, format=I420, width={}, height={}, framerate={}/1 ! ",
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

    let (enc, parse, cvt) = ("x264enc", "h264parse", "v4l2convert");

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
                .framerate(gst::Fraction::new(fps as _, 1))
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
        .property("bitrate", 2500_u32)
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
        .property("peak-kbps", 2500_u32)
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

    // * Draw callback
    struct SharedPtr<T>(*mut T);
    unsafe impl<T> Send for SharedPtr<T> {}
    unsafe impl<T> Sync for SharedPtr<T> {}
    impl<T> Clone for SharedPtr<T> {
        fn clone(&self) -> Self {
            Self(self.0)
        }
    }

    let draw_frame = SharedPtr(&mut draw_frame as *mut F);
    video_overlay.connect("draw", false, move |args| {
        unsafe {
            (*draw_frame.clone().0)(
                args[1].get::<cairo::Context>().unwrap(),
                width as _,
                height as _,
            );
        }
        None
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

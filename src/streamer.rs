extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate gstreamer_audio as gst_audio;
extern crate gstreamer_video as gst_video;

use std::str::FromStr;

use gst::{prelude::*, Caps, ElementFactory};
use rand::Rng;

pub fn stream<F>(
    size: (usize, usize),
    fps: usize,
    video_bitrate: usize,
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
    let rtmp_sink = ElementFactory::make("rtmpsink")
        .property("location", rtmp_uri)
        .build()
        .unwrap();

    // * Audio
    let audio_source = gst_app::AppSrc::builder()
        .is_live(true)
        .caps(
            // &gst_audio::AudioInfo::builder(gst_audio::AudioFormat::S32be, 44100, 2)
            //     .build()
            //     .unwrap()
            //     .to_caps()
            //     .unwrap(),
            &Caps::from_str(
                "audio/x-raw-float,width=32,depth=32,endianness=1234,rate=44100,channels=2",
            )
            .unwrap(),
        )
        .build();
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
            audio_source.upcast_ref(),
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
    gst::Element::link_many([audio_source.upcast_ref(), &audio_encoder, &mux]).unwrap();

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

    // * Audio callback
    audio_source.set_callbacks(
        gst_app::AppSrcCallbacks::builder()
            .need_data(|src, _| {
                let mut buffer = gst::Buffer::with_size(1024).unwrap();
                {
                    let mut buffer = buffer.get_mut().unwrap().map_writable().unwrap();
                    let data = buffer.as_mut_slice();
                    for sample in data.chunks_exact_mut(4) {
                        sample.clone_from_slice(
                            &rand::thread_rng()
                                .gen_range::<f32, _>(-1.0..1.0)
                                .to_be_bytes(),
                        )
                    }
                }
                src.push_buffer(buffer).unwrap();
            })
            .build(),
    );

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in pipeline.bus().unwrap().iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                panic!("Element {:?}:\n{}", err.src(), err);
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();
}

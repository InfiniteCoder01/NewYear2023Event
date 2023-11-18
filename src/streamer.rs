extern crate gstreamer as gst;
extern crate gstreamer_app as gst_app;
extern crate gstreamer_audio as gst_audio;
extern crate gstreamer_video as gst_video;

use gst::{prelude::*, Caps, ElementFactory};

pub fn stream<F>(
    size: (usize, usize),
    fps: usize,
    video_bitrate: usize,
    audio_samplerate: usize,
    audio_bitrate: usize,
    rtmp_uri: &str,
    draw_frame: F,
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
    //         "appsrc ! audioconvert ! queue ! voaacenc bitrate=128000 ! mux."
    //     ),
    //     width, height, fps,
    //     width, height, fps,
    //     rtmp_uri
    // );

    gst::init().unwrap();
    let pipeline = gst::Pipeline::default();

    // let (enc, parse, cvt) = ("x264enc", "h264parse", "v4l2convert");
    let (enc, parse, cvt) = ("x264enc", "h264parse", "videoconvert");

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
    // let audio_source = gst_app::AppSrc::builder()
    //     .is_live(true)
    //     .caps(
    //         &gst_audio::AudioInfo::builder(gst_audio::AudioFormat::F32le, audio_samplerate as _, 2)
    //             .build()
    //             .unwrap()
    //             .to_caps()
    //             .unwrap(),
    //     )
    //     .format(gst::Format::Time)
    //     .build();
    let audio_source = ElementFactory::make("audiotestsrc").build().unwrap();
    let audio_converter = ElementFactory::make("audioconvert").build().unwrap();
    let audio_queue = ElementFactory::make("queue")
        .property_from_str("leaky", "upstream")
        .build()
        .unwrap();
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
            // &audio_converter,
            // &audio_queue,
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
    gst::Element::link_many([
        audio_source.upcast_ref(),
        // &audio_converter,
        // &audio_queue,
        &audio_encoder,
        &mux,
    ])
    .unwrap();

    // let audio_mixer = std::sync::Arc::new(std::sync::Mutex::new(Mixer::default()));

    // * Draw callback
    // let callback_audio_mixer = audio_mixer.clone();
    let draw_frame = std::sync::Mutex::new(draw_frame);
    video_overlay.connect("draw", false, move |args| {
        println!("Frame!");
        draw_frame.lock().unwrap()(
            args[1].get::<cairo::Context>().unwrap(),
            width as _,
            height as _,
            // &mut callback_audio_mixer.clone().lock().unwrap(),
        );
        None
    });

    // * Audio callback
    // audio_source.set_callbacks(
    //     gst_app::AppSrcCallbacks::builder()
    //         .need_data(move |src, _length| {
    //             // let mut audio_mixer = audio_mixer.lock().unwrap();
    //             // let mut samples = Vec::new();
    //             // audio_mixer.voices.retain_mut(|voice| {
    //             //     match voice.next_frame() {
    //             //         Ok(minimp3::Frame {
    //             //             data,
    //             //             sample_rate,
    //             //             channels,
    //             //             ..
    //             //         }) => {
    //             //             for sample in data
    //             //                 .chunks_exact(channels)
    //             //                 .step_by((sample_rate as usize / audio_samplerate).max(1))
    //             //             {
    //             //                 samples
    //             //                     .extend(sample.iter().map(|sample| *sample as f32 / 32767.0));
    //             //             }
    //             //         }
    //             //         Err(minimp3::Error::Eof) => return false,
    //             //         Err(e) => panic!("{:?}", e),
    //             //     }
    //             //     let buffer = gst::Buffer::from_slice(unsafe {
    //             //         std::slice::from_raw_parts(samples.as_ptr() as *const u8, samples.len() * 4)
    //             //     });
    //             //     src.push_buffer(buffer).unwrap();
    //             //     true
    //             // });
    //         })
    //         .build(),
    // );

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in pipeline.bus().unwrap().iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                panic!(
                    "Element {}:\n{}",
                    err.src().map_or(String::from("None"), |elemen| elemen
                        .name()
                        .as_str()
                        .to_owned()),
                    err
                );
            }
            MessageView::Warning(warning) => {
                eprintln!(
                    "Warning from element {}:\n{}",
                    warning.src().map_or(String::from("None"), |elemen| elemen
                        .name()
                        .as_str()
                        .to_owned()),
                    warning
                );
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();
}

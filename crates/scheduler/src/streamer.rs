pub extern crate gstreamer as gst;
pub extern crate gstreamer_audio as gst_audio;
pub extern crate gstreamer_base as gst_base;
pub extern crate gstreamer_video as gst_video;

use gst::{parse_launch, prelude::*, Element, Pipeline};
use std::sync::Mutex;

static VIDEO_SOURCE: Mutex<Option<Element>> = Mutex::new(None);
static VIDEO_SWITCH: Mutex<Option<Element>> = Mutex::new(None);

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
    let (width, height) = size;
    let (videocvt, audioenc) = if virtual_mode {
        ("videoconvert", "faac")
    } else {
        (/*"v4l2convert"*/"autovideoconvert", "voaacenc")
    };

    // filesrc location="/home/infinitecoder/Downloads/file_example_MP4_1280_10MG.mp4" ! qtdemux name=demux
    // demux.audio_0 ! audioconvert ! pulsesink
    // demux.video_0 ! input-selector name=video_switch
    // rawvideoparse use-sink-caps=false format="RGBx" width={width} height={height} !
    let mut pipeline = format!(
        r#"
            videotestsrc pattern=black !
            cairooverlay name="video_overlay" !
            video/x-raw, width={width}, height={height}, format=BGRx !
            {videocvt} ! video/x-raw, format=I420 ! video_switch.

            input-selector name=video_switch !
        "#
    );
    if virtual_mode {
        pipeline += "autovideosink";
    } else {
        pipeline += &format!(
            r#"
                x264enc key-int-max=30 bitrate={video_bitrate} speed-preset=ultrafast ! h264parse !
                flvmux streamable=true name=mux ! rtmp2sink location={rtmp_uri}

                pulsesrc ! {audioenc} bitrate={audio_bitrate} ! mux.
            "#
        );
    };

    gst::init().unwrap();
    let pipeline = parse_launch(&pipeline)
        .unwrap()
        .downcast::<Pipeline>()
        .unwrap();

    let video_overlay = pipeline.by_name("video_overlay").unwrap();

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

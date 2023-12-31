pub extern crate gstreamer as gst;
pub extern crate gstreamer_audio as gst_audio;
pub extern crate gstreamer_base as gst_base;
pub extern crate gstreamer_video as gst_video;

use super::*;
use gst::{parse_launch, prelude::*, Element, Pipeline};

#[derive(Clone, Debug)]
pub struct BackgroundController {
    pipeline: Pipeline,
    file_bin: Element,
    file_src: Element,
    video_switch: Element,
}

impl BackgroundController {
    pub fn set_file_source(&self, location: &str) {
        self.file_src.set_property("location", location);
        self.pipeline.set_state(gst::State::Ready).unwrap();
        self.file_bin.set_locked_state(false);
        self.video_switch.set_property(
            "active-pad",
            &self.video_switch.static_pad("sink_1").unwrap(),
        );
        self.pipeline.set_state(gst::State::Playing).unwrap();
    }

    pub fn disable_background_video(&self) {
        self.pipeline.set_state(gst::State::Ready).unwrap();
        self.file_bin.set_locked_state(true);
        self.video_switch.set_property(
            "active-pad",
            &self.video_switch.static_pad("sink_0").unwrap(),
        );
        self.pipeline.set_state(gst::State::Playing).unwrap();
    }
}

pub fn stream<F>(
    size: (usize, usize),
    audio_bitrate: usize,
    h264_level: &str,
    rtmp_uri: &str,
    draw_frame: F,
    virtual_mode: bool,
) where
    F: FnMut(&BackgroundController, cairo::Context, f64, f64) + Send + Sync + 'static,
{
    let (width, height) = size;
    let (videoconvert, audioenc) = if virtual_mode {
        ("videoconvert", "faac")
    } else {
        ("v4l2convert", "voaacenc")
    };

    let mut pipeline = format!(
        // file_demux. ! audioconvert ! audioresample ! pulsesink
        // location="/home/infinitecoder/Downloads/file_example_MP4_1280_10MG.mp4"

        // file_demux.src_1 ! audioconvert ! audioresample ! pulsesink
        r#"
        videotestsrc pattern=black ! video_switch.sink_0

        bin (name=file_bin
                filesrc name=file_src ! decodebin name=file_demux
                file_demux. ! audioconvert ! audioresample ! pulsesink
                file_demux. ! {videoconvert} ! videoscale ! queue
            ) ! video_switch.sink_1

            input-selector name=video_switch !
            cairooverlay name=video_overlay !
            video/x-raw, width={width}, height={height}, format=RGB16, framerate=30/1 !
        "#
    );
    if virtual_mode {
        pipeline += "glimagesink";
    } else {
        pipeline += &format!(
            r#"
                v4l2h264enc ! video/x-h264, level=(string){h264_level} ! h264parse ! queue !
                flvmux streamable=true name=mux ! rtmp2sink location="{rtmp_uri}"

                pulsesrc ! {audioenc} bitrate={audio_bitrate} ! queue ! mux.
            "#
        );
    };

    gst::init().unwrap();
    let pipeline = parse_launch(&pipeline)
        .unwrap()
        .downcast::<Pipeline>()
        .unwrap();

    // * Video Switch
    let background = BackgroundController {
        pipeline: pipeline.clone(),
        file_bin: pipeline.by_name("file_bin").unwrap(),
        file_src: pipeline.by_name("file_src").unwrap(),
        video_switch: pipeline.by_name("video_switch").unwrap(),
    };

    background.file_bin.set_locked_state(true);
    log_error!("Failed to start the flow: {}!"; pipeline.set_state(gst::State::Playing));

    background
        .video_switch
        .set_property("active-pad", background.video_switch.static_pad("sink_0"));

    // * Draw callback
    let video_overlay = pipeline.by_name("video_overlay").unwrap();
    let draw_frame = std::sync::Mutex::new(draw_frame);
    video_overlay.connect("draw", false, move |args| {
        draw_frame.lock().unwrap()(
            &background,
            args[1].get::<cairo::Context>().unwrap(),
            width as _,
            height as _,
        );
        None
    });

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

    pipeline.set_state(gst::State::Null).unwrap();
}

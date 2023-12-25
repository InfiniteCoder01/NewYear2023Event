pub extern crate gstreamer as gst;
pub extern crate gstreamer_audio as gst_audio;
pub extern crate gstreamer_base as gst_base;
pub extern crate gstreamer_video as gst_video;

use super::*;
use gst::{parse_bin_from_description, parse_launch, prelude::*, Bin, Element, Pipeline};

pub struct BackgroundController {
    pipeline: Pipeline,
    file_src: Element,
    video_switch: Element,
}

impl BackgroundController {
    pub fn set_file_source(&self, location: &str) {
        // self.file_pipeline
        //     .by_name("file_src")
        //     .unwrap()
        //     .set_property("location", location);

        // self.pipeline
        //     .by_name("dummy")
        //     .unwrap()
        //     .unlink(&self.video_switch);
        // log_error!("Failed to add file to pipeline: {}!"; self.pipeline.add(&self.file_pipeline));
        // log_error!("Failed to link file to pipeline: {}!"; self.file_pipeline.src_pads().last().unwrap().link(self.video_switch.sink_pads().last().unwrap()));
        // self.video_switch
        //     .set_property("active-pad", self.video_switch.sink_pads().last().unwrap());
    }

    pub fn disable_background_video(&self) {
        self.video_switch
            .set_property("active-pad", self.video_switch.static_pad("sink_0"));
        log_error!("Failed to remove file from pipeline: {}!"; self.pipeline.remove(&self.file_pipeline));
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
    let audioenc = if virtual_mode { "faac" } else { "voaacenc" };

    let mut pipeline = format!(
        // file_demux. ! audioconvert ! audioresample ! pulsesink
        // location="/home/infinitecoder/Downloads/file_example_MP4_1280_10MG.mp4"
        r#"
            videotestsrc pattern=black ! video_switch.sink_0

            filesrc name=file_src !
            decodebin name=file_demux ! videoconvert ! video_switch.sink_1

            input-selector name=video_switch !
            cairooverlay name=video_overlay !
            video/x-raw, width={width}, height={height}, format=RGB16 !
        "#
    );
    if virtual_mode {
        pipeline += "glimagesink";
    } else {
        pipeline += &format!(
            r#"
                v4l2h264enc ! video/x-h264, level=(string){h264_level} ! h264parse !
                flvmux streamable=true name=mux ! rtmp2sink location="{rtmp_uri}"

                pulsesrc ! {audioenc} bitrate={audio_bitrate} ! mux.
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
        file_src: pipeline.by_name("file_src").unwrap(),
        video_switch: pipeline.by_name("video_switch").unwrap(),
    };

    log_error!("Failed to start the flow: {}!"; pipeline.set_state(gst::State::Playing));

    background
        .video_switch
        .set_property("active-pad", background.video_switch.static_pad("sink_0"));
    background.file_src.set_locked_state(true);

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

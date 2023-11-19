use crate::streamer::*;
use glib::once_cell::sync::Lazy;
use gst_base::subclass::prelude::*;

pub static mut MIXER: Option<Arc<Mutex<Mixer>>> = None;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct MixerSource {
        // buffer: std::sync::Mutex<Vec<u8>>,
        time: std::sync::Mutex<gst::ClockTime>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MixerSource {
        const NAME: &'static str = "MixerSource";
        type Type = super::MixerSource;
        type ParentType = gst_base::PushSrc;
        type Interfaces = ();
    }

    impl ObjectImpl for MixerSource {}
    impl GstObjectImpl for MixerSource {}
    impl ElementImpl for MixerSource {
        fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
            static ELEMENT_METADATA: Lazy<gst::subclass::ElementMetadata> = Lazy::new(|| {
                gst::subclass::ElementMetadata::new(
                    "Mixer Source",
                    "Source/Audio",
                    "Mixer Source for my pipeline",
                    "InfiniteCoder <nayka.0.lobach.01@gmail.com>",
                )
            });

            Some(&*ELEMENT_METADATA)
        }

        fn pad_templates() -> &'static [gst::PadTemplate] {
            static PAD_TEMPLATES: Lazy<Vec<gst::PadTemplate>> = Lazy::new(|| {
                // * FORMAT
                let caps = &gst_audio::AudioInfo::builder(gst_audio::AudioFormat::S16le, 44100, 2)
                    .build()
                    .unwrap()
                    .to_caps()
                    .unwrap();

                let src_pad_template = gst::PadTemplate::new(
                    "src",
                    gst::PadDirection::Src,
                    gst::PadPresence::Always,
                    caps,
                )
                .unwrap();

                vec![src_pad_template]
            });

            PAD_TEMPLATES.as_ref()
        }
    }

    impl BaseSrcImpl for MixerSource {
        fn is_seekable(&self) -> bool {
            false
        }

        fn times(
            &self,
            buffer: &gst::BufferRef,
        ) -> (Option<gst::ClockTime>, Option<gst::ClockTime>) {
            let duration =
                gst::ClockTime::from_seconds_f32(buffer.size() as f32 / 2.0 / 2.0 / 44100.0);
            let mut current_time = self.time.lock().unwrap();
            let start = *current_time;
            *current_time = start + duration;
            (Some(start), Some(*current_time))
        }
    }

    impl PushSrcImpl for MixerSource {
        fn create(
            &self,
            _buffer: Option<&mut gst::BufferRef>,
        ) -> Result<gst_base::subclass::base_src::CreateSuccess, gst::FlowError> {
            let mut audio_mixer = unsafe { MIXER.as_ref() }.unwrap().lock().unwrap();
            let mut samples = vec![0_i16; 2048];
            if let Some(voice) = &mut audio_mixer.voice {
                match voice.next_frame() {
                    Ok(minimp3::Frame {
                        data,
                        sample_rate: _,
                        channels: _,
                        ..
                    }) => samples = data,
                    Err(minimp3::Error::Eof) => audio_mixer.voice = None,
                    Err(e) => {
                        eprintln!("{:?}", e);
                        audio_mixer.voice = None;
                    }
                }
            }
            dbg!(samples.len());
            let buffer = gst::Buffer::from_slice(unsafe {
                std::slice::from_raw_parts(samples.as_ptr() as *const u8, samples.len() * 2)
            });
            Ok(gst_base::subclass::base_src::CreateSuccess::NewBuffer(
                buffer,
            ))
        }

        // fn fill(&self, buffer: &mut gst::BufferRef) -> Result<gst::FlowSuccess, gst::FlowError> {
        //     let mut audio_mixer = unsafe { MIXER.as_ref() }.unwrap().lock().unwrap();
        //     let mut inner_buffer = self.buffer.lock().unwrap();
        //     while inner_buffer.len() < buffer.size() {
        //         if let Some(voice) = &mut audio_mixer.voice {
        //             match voice.next_frame() {
        //                 Ok(minimp3::Frame {
        //                     data,
        //                     sample_rate: _,
        //                     channels: _,
        //                     ..
        //                 }) => {
        //                     inner_buffer
        //                         .extend(data.iter().flat_map(|sample| sample.to_le_bytes()));
        //                     continue;
        //                 }
        //                 Err(minimp3::Error::Eof) => {
        //                     audio_mixer.voice = None;
        //                 }
        //                 Err(e) => {
        //                     eprintln!("{:?}", e);
        //                     audio_mixer.voice = None;
        //                 }
        //             }
        //         }
        //         let need = buffer.size() - inner_buffer.len();
        //         inner_buffer.extend(std::iter::repeat(0).take(need));
        //     }
        //     let need = buffer.size();
        //     buffer
        //         .map_writable()
        //         .unwrap()
        //         .as_mut_slice()
        //         .copy_from_slice(&inner_buffer[..need]);
        //     inner_buffer.drain(..need);
        //     Ok(gst::FlowSuccess::Ok)
        // }
    }
}

glib::wrapper! {
    pub struct MixerSource(ObjectSubclass<imp::MixerSource>) @extends gst_base::PushSrc, gst_base::BaseSrc, gst::Element, gst::Object;
}

impl MixerSource {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Default for MixerSource {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "mixersource",
        gst::Rank::Primary,
        MixerSource::static_type(),
    )
}

gst::plugin_define!(
    mixersource,
    "Mixer Source for my pipeline",
    register,
    "1.0.0",
    "MPL",
    "mixersource",
    "mixersource",
    env!("CARGO_PKG_REPOSITORY")
);

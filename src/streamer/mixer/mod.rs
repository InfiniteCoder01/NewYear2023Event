pub mod source;

#[derive(Default)]
pub struct Mixer {
    pub voice: Option<minimp3::Decoder<std::fs::File>>,
}

impl Mixer {
    pub fn play(&mut self, path: &str) {
        self.voice = Some(minimp3::Decoder::new(std::fs::File::open(path).unwrap()));
    }

    pub fn silent(&self) -> bool {
        self.voice.is_none()
    }
}

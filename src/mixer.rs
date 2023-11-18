// #[derive(Default)]
// pub struct Mixer {
//     voices: Vec<minimp3::Decoder<std::fs::File>>,
// }

// impl Mixer {
//     pub fn play(&mut self, path: &str) {
//         self.voices
//             .push(minimp3::Decoder::new(std::fs::File::open(path).unwrap()));
//     }

//     pub fn silent(&self) -> bool {
//         self.voices.is_empty()
//     }
// }

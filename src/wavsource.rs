use std::io::{Read, Result, Seek, SeekFrom};

use hound::WavReader;
use symphonia_core::io::MediaSource;

use crate::timestretch::apply_time_stretch;

pub struct WavSource<'a> {
    iterator: Box<dyn Iterator<Item = u8> + 'a + Send + Sync>,
}

#[allow(clippy::unnecessary_wraps)]
fn completion_2x(cum: &mut i16, v: i16) -> Option<[i16; 2]> {
    let comp = i32::from(*cum) + (i32::from(v) - i32::from(*cum)) / 2;
    *cum = v;

    #[allow(clippy::cast_possible_truncation)]
    Some([comp as i16, v])
}

impl WavSource<'_> {
    pub fn new<R: Seek + Read>(reader: &mut R) -> (Self, u32) {
        let mut wave = WavReader::new(reader).unwrap();
        let data: Vec<i16> = wave.samples().map(|v| v.unwrap()).collect();

        let sample_rate = wave.spec().sample_rate;
        let channels = wave.spec().channels as usize;

        let data = apply_time_stretch(&data, channels, sample_rate);

        if sample_rate <= 24000 {
            (
                Self {
                    iterator: Box::new(
                        data.into_iter()
                            .scan(0, completion_2x)
                            .flatten()
                            .flat_map(|v| f32::to_le_bytes(f32::from(v) / f32::from(i16::MAX))),
                    ),
                },
                sample_rate * 2,
            )
        } else {
            (
                Self {
                    iterator: Box::new(
                        data.into_iter()
                            .flat_map(|v| f32::to_le_bytes(f32::from(v) / f32::from(i16::MAX))),
                    ),
                },
                sample_rate,
            )
        }
    }
}

impl Read for WavSource<'_> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = 0;

        for (b, d) in buf.iter_mut().zip(&mut self.iterator) {
            *b = d;
            len += 1;
        }

        Ok(len)
    }
}

impl Seek for WavSource<'_> {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        unimplemented!();
    }
}

impl MediaSource for WavSource<'_> {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

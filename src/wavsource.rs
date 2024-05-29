use std::io::{Read, Result, Seek, SeekFrom};

use hound::WavReader;
use symphonia_core::io::MediaSource;

pub struct WavSource<'a> {
    iterator: Box<dyn Iterator<Item = u8> + 'a + Send + Sync>,
}

#[allow(clippy::unnecessary_wraps)]
fn completion_24k_to_48k(cum: &mut i16, v: i16) -> Option<[i16; 2]> {
    let comp = i32::from(*cum) + (i32::from(v) - i32::from(*cum)) / 2;
    *cum = v;

    #[allow(clippy::cast_possible_truncation)]
    Some([comp as i16, v])
}

impl<'a> WavSource<'a> {
    pub fn new<R: Seek + Read>(reader: &mut R) -> Self {
        let data: Vec<i16> = WavReader::new(reader)
            .unwrap()
            .samples()
            .map(|v| v.unwrap())
            .collect();

        Self {
            iterator: Box::new(
                data.into_iter()
                    .scan(0, completion_24k_to_48k)
                    .flatten()
                    .flat_map(|v| f32::to_le_bytes(f32::from(v) / f32::from(i16::MAX))),
            ),
        }
    }
}

impl<'a> Read for WavSource<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = 0;

        for (b, d) in buf.iter_mut().zip(&mut self.iterator) {
            *b = d;
            len += 1;
        }

        Ok(len)
    }
}

impl<'a> Seek for WavSource<'a> {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        unimplemented!();
    }
}

impl<'a> MediaSource for WavSource<'a> {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

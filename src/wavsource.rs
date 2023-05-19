use std::io::{Read, Result, Seek, SeekFrom};
use std::slice;

use songbird::input::{reader::MediaSource, Codec, Container, Input, Reader};
use wav::bit_depth::BitDepth;

pub fn wav_reader<R: Read + Seek>(reader: &mut R) -> Input {
    Input::new(
        false,
        Reader::Extension(Box::new(WavSource::new(reader))),
        Codec::Pcm,
        Container::Raw,
        None,
    )
}

pub struct WavSource {
    iterator: slice::Iter<'static, u8>,
}

impl WavSource {
    pub fn new<R: Seek + Read>(reader: &mut R) -> Self {
        let (_, BitDepth::Sixteen(data)) = wav::read(reader).unwrap() else {
            unimplemented!();
        };

        // 24000Hz -> 48000Hz
        // TODO: You can save the queue memory half if you move this implementation to the Read.
        let data: Vec<i16> = data
            .iter()
            .scan(0, |cum, v| {
                let comp = i32::from(*cum) + (i32::from(*v) - i32::from(*cum)) / 2;
                *cum = *v;

                #[allow(clippy::cast_possible_truncation)]
                Some([comp as i16, *v])
            })
            .flatten()
            .collect();

        let data = unsafe {
            let mut data = std::mem::ManuallyDrop::new(data);
            slice::from_raw_parts(data.as_mut_ptr() as *mut u8, data.len() * 2)
        };

        Self { iterator: data.into_iter() }
    }
}

impl Read for WavSource {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = 0;

        for (b, d) in buf.iter_mut().zip(self.iterator.next()) {
            *b = *d;
            len += 1;
        }

        Ok(len)
    }
}

impl Seek for WavSource {
    fn seek(&mut self, _pos: SeekFrom) -> Result<u64> {
        unimplemented!();
    }
}

impl MediaSource for WavSource {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

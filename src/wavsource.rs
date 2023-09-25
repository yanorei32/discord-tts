use std::io::{Read, Result, Seek, SeekFrom};

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

pub struct WavSource<'a> {
    iterator: Box<dyn Iterator<Item = u8> + 'a>,
}

#[allow(clippy::unnecessary_wraps)]
fn completion(cum: &mut i16, v: i16) -> Option<[i16; 2]> {
    let comp = i32::from(*cum) + (i32::from(v) - i32::from(*cum)) / 2;
    *cum = v;

    #[allow(clippy::cast_possible_truncation)]
    Some([comp as i16, v])
}

impl<'a> WavSource<'a> {
    pub fn new<R: Seek + Read>(reader: &mut R) -> Self {
        let (_, BitDepth::Sixteen(data)) = wav::read(reader).unwrap() else {
            unimplemented!();
        };

        Self {
            iterator: Box::new(
                data.clone()
                    .into_iter()
                    .scan(0, completion)
                    .flatten()
                    .flat_map(i16::to_le_bytes),
            ),
        }
    }
}

impl<'a> Read for WavSource<'a> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = 0;

        for (b, d) in buf.iter_mut().zip(self.iterator.next()) {
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

// This impl are required by MediaSource,
// but this application does not use MediaSource.
unsafe impl<'a> Send for WavSource<'a> {}
unsafe impl<'a> Sync for WavSource<'a> {}

impl<'a> MediaSource for WavSource<'a> {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

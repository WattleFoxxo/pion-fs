use super::{FileSystem, Error};
use super::stream::Stream;
use super::dir_entry::{DirEntry, DIR_ENTRY_SIZE, REMOVED_ENTRY,FREE_ENTRY};

use super::lfn::{Lfn, LfnBuilder};



pub struct DirIterator<'a, F> {
    stream: Stream,
    fs: &'a F,
}

impl <'a, F> DirIterator<'a, F> {
    pub fn new(fs: &'a F, cluster: u32) -> Self {
        Self {
            stream: Stream::open(cluster),
            fs,
        }
    }
}

impl <'a, F: FileSystem> Iterator for DirIterator<'a, F> {
    type Item = Result<(DirEntry<'a, F>, Option<Lfn>), Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut lfn = Lfn::new();
        let mut lfn_builder = LfnBuilder::new(&mut lfn);

        loop {
            let mut buf = [0u8; DIR_ENTRY_SIZE];
            let pos = self.stream;

            match self.stream.read(self.fs, &mut buf) {
                Ok(len) => {
                    if len == 0 {
                        // no more dir entries in stream
                        return None;
                    }
                },
                Err(e) => {
                    return Some(Err(e));
                }
            }

            if buf[0] == FREE_ENTRY {
                // no more dir entry at all
                return None;
            }

            if buf[0] == REMOVED_ENTRY {
                // removed dir entry
                lfn_builder.reset();
                continue;
            }

            if lfn_builder.process(pos.clone(), &buf) {
                continue;
            }

            if let Some((lfn_pos, lfn_crc)) = lfn_builder.build() {
                if let Ok(entry) = DirEntry::from_raw_data(buf, self.fs, pos, Some((lfn_pos, lfn_crc))) {
                    if entry.is_dir() || entry.is_file() {
                        return Some(Ok((entry, Some(lfn))));
                    }
                }
            } else {
                if let Ok(entry) = DirEntry::from_raw_data(buf, self.fs, pos, None) {
                    if entry.is_dir() || entry.is_file() {
                        return Some(Ok((entry, None)));
                    }
                }
            }

            lfn_builder.reset();
        }
    }
}

impl <'a, F: FileSystem> DirIterator<'a, F> {
    pub fn find_free_space(fs: &F, cluster: u32, dir_entry_count: usize) -> Result<Stream, Error> {
        let mut stream = Stream::open(cluster);
        let mut pos_to_write = stream;
        let mut count = 0;

        loop {
            let mut buf = [0u8; DIR_ENTRY_SIZE];
            let pos = stream;
            let len = stream.read(fs, &mut buf)?;

            if len == 0 {
                // no more dir entries in stream
                return Ok(pos);
            }

            assert!(len == DIR_ENTRY_SIZE);

            if buf[0] == FREE_ENTRY  {
                return Ok(pos);
            }

            if buf[0] == REMOVED_ENTRY {
                if count == 0 {
                    pos_to_write = pos;
                }

                count += 1;

                if count == dir_entry_count {
                    return Ok(pos_to_write);
                }
            } else {
                count = 0;
            }
        }
    }
}


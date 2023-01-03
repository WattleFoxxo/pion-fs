use super::{FileSystem, Error};
use super::stream::Stream;
use super::dir_entry::DirEntry;

pub struct File<'a, F> {
    fs: &'a F,
    dir_entry: DirEntry<'a, F>,
    stream: Stream,
    offset: u32,
    is_dirty: bool,
}
enum SeekFrom {
    Start(usize),
    Current(isize),
    End(isize),
}

impl <'a, F: FileSystem> File<'a, F> {
    pub fn open(dir_entry: DirEntry<'a, F>) -> Result<Self, Error> {
        if !dir_entry.is_file() {
            return Err(Error::NotFile);
        }

        let fs = dir_entry.fs();

        Ok(Self {
            stream: Stream::open(dir_entry.cluster()),
            dir_entry,
            offset: 0,
            is_dirty: false,
            fs,
        })
    }

    pub fn read(&mut self, buf: &mut[u8]) -> Result<usize, Error> {
        let bytes_left_in_file = self.dir_entry.size() - self.offset;
        let len_to_read = core::cmp::min(buf.len(), bytes_left_in_file as usize);
        let len = self.stream.read(self.fs, &mut buf[..len_to_read])?;
        self.offset += len as u32;
        Ok(len)
    }

    fn seek(&mut self, pos: SeekFrom) -> Result<usize, Error> {
        let offset_from_origin = match pos {
            SeekFrom::Current(offset) => {
                self.offset as isize + offset
            },
            SeekFrom::Start(offset) => {
                offset as isize
            },
            SeekFrom::End(offset) => {
                self.dir_entry.size() as isize + offset
            },
        };

        let pos = {
            if offset_from_origin < 0 {
                0
            } else if offset_from_origin as u32 > self.dir_entry.size() {
                self.dir_entry.size()
            } else {
                offset_from_origin as u32
            }
        };

        //self.stream.seek(fs, pos)
        todo!();
    }

    /*
    pub fn seek(&mut self, fs: &Fs, pos: usize) -> Result<usize, Error> {
        let cluster_size = fs.sectors_in_cluster() as usize * fs.sector_size();
        let pos_cluster = pos / cluster_size;
        let current_pos_cluster = self.offset_from_origin / cluster_size;

        let mut start_cluster;
        let clusters_to_skip;

        if pos_cluster < current_pos_cluster {
            start_cluster = self.origin;
            clusters_to_skip = pos_cluster;
        } else {
            start_cluster = self.current;
            clusters_to_skip = current_pos_cluster - pos_cluster;
        }
        // skip clusters if need
        for _ in 0..clusters_to_skip {
            match FatTable::get(fs, start_cluster)? {
                ClusterValue::Next(cluster) => {
                    start_cluster = cluster;
                },
                ClusterValue::Last => {
                    return Err(Error::UnexpectedEndOfFile);
                },
                ClusterValue::Free | ClusterValue::Bad => {
                    return Err(Error::UnexpectedClusterValue);
                }
            }
        }

        let start_cluster_sector = fs.cluster_to_sector(start_cluster)?;
        self.current = start_cluster;
        self.sector = start_cluster_sector + ((pos % cluster_size) / fs.sector_size()) as u32;
        self.last_sector_in_cluster = start_cluster_sector + fs.sector_size() as u32;
        self.offset_in_sector = pos / fs.sector_size();
        self.offset_from_origin = pos;
        Ok(self.offset_from_origin)
    }
    */

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let len = self.stream.write(self.fs, buf)?;
        self.offset += len as u32;

        if self.offset > self.dir_entry.size() {
            self.dir_entry.set_size(self.offset);
            self.is_dirty = true;
        }
        Ok(len)
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        if self.is_dirty {
            self.dir_entry.flush()?;
            self.is_dirty = false;
        }
        Ok(())
    }

    pub fn truncate(&mut self) -> Result<u32, Error> {
        self.stream.truncate(self.fs)?;
        self.dir_entry.set_size(self.offset);
        self.is_dirty = true;
        Ok(self.offset)
    }

    pub fn close(mut self) -> Result<(), Error> {
        self.flush()
    }
}
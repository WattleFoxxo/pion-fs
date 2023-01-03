use super::{FileSystem, Error, ClusterValue};
use super::fat_table;

#[derive(Clone, Copy)]
pub struct Stream {
    cluster: u32,
    offset: usize,
}

impl Stream {
    pub fn create<F: FileSystem>(fs: &F) -> Result<Self, Error> {
        let cluster = fat_table::create(fs)?;
        Ok(Self::open(cluster))
    }

    pub fn open(cluster: u32) -> Self {
        Self {
            cluster,
            offset: 0,
        }
    }

    pub fn remove<F: FileSystem>(fs: &F, cluster: u32) -> Result<(), Error> {
        fat_table::remove(fs, cluster)
    }

    pub fn write<F: FileSystem>(&mut self, fs: &F, buf: &[u8]) -> Result<usize, Error> {
        let mut bytes_written = 0;

        while bytes_written != buf.len() {
            if self.offset == fs.cluster_size() {
                match fs.fat_table_get(self.cluster)? {
                    ClusterValue::Next(cluster) => {
                        self.cluster = cluster;
                    },
                    ClusterValue::Last => {
                        // extend cluster chain
                        let cluster = fat_table::extend(fs, self.cluster)?;
                        self.cluster = cluster;
                    },
                    ClusterValue::Bad | ClusterValue::Free => {
                        return Err(Error::UnexpectedClusterValue);
                    },
                }

                self.offset = 0;
            }

            let len = fs.write(self.cluster, self.offset, &buf[bytes_written..])?;
            bytes_written += len;
            self.offset += len;
        }
        Ok(bytes_written)
    }

    pub fn read<F: FileSystem>(&mut self, fs: &F, buf: &mut [u8]) -> Result<usize, Error> {
        let mut bytes_read = 0;

        while bytes_read != buf.len() {
            if self.offset == fs.cluster_size() {
                match fs.fat_table_get(self.cluster)? {
                    ClusterValue::Next(cluster) => {
                        self.cluster = cluster;
                    },
                    ClusterValue::Last => {
                        break;
                    },
                    ClusterValue::Bad | ClusterValue::Free => {
                        return Err(Error::UnexpectedClusterValue);
                    },
                }

                self.offset = 0;
            }

            let len = fs.read(self.cluster, self.offset, &mut buf[bytes_read..])?;
            bytes_read += len;
            self.offset += len;
        }
        Ok(bytes_read)
    }

    pub fn truncate<F: FileSystem>(&mut self, fs: &F) -> Result<(), Error> {
        fat_table::truncate(fs, self.cluster)
    }

    pub fn cluster(&self) -> u32 {
        self.cluster
    }

    pub fn is_equal(&self, pos: &Stream) -> bool {
        self.cluster == pos.cluster && self.offset == pos.offset
    }
}
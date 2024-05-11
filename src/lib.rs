#![no_std]

pub mod dir;
mod dir_entry;
mod stream;
pub mod file;
mod fat_table;
mod path;
mod dir_iterator;
pub mod fs;
pub mod block_device;
mod lfn;

#[derive(Debug)]
pub enum Error {
    NotFile,
    NotDir,
    NotFound,
    UnexpectedClusterValue,
    UnexpectedEndOfFile,
    InvalidClusterNumber,
    NoFreeCluster,
    DirNotEmpty,
    ObjectAlreadyExist,
    DeviceIO,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32
}

pub enum ClusterValue {
    Free,
    Next(u32),
    Last,
    Bad,
}

pub trait FileSystem {
    fn root_cluster(&self) -> u32;
    fn cluster_count(&self) -> u32;
    fn cluster_size(&self) -> usize;
    fn read(&self, cluster: u32, offset: usize, buf: &mut [u8]) -> Result<usize, Error>;
    fn write(&self, cluster: u32, offset: usize, buf: &[u8]) -> Result<usize, Error>;
    fn fat_table_get(&self, cluster: u32) -> Result<ClusterValue, Error>;
    fn fat_table_set(&self, cluster: u32, value: ClusterValue) -> Result<(), Error>;
    fn flush(&self) -> Result<(), Error>;
}

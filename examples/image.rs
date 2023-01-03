use pion_fs::block_device::BlockDevice;
use std::{fs::File, io::SeekFrom, io::Seek, io::Read, io::Write, fs::OpenOptions};
use core::cell::RefCell;
use std::env;

pub struct Image {
    file: RefCell<File>,
    lba_count: u32,
    lba_size: usize,
}

impl Image {
    pub fn new(path: &str, lba_size: usize) -> Self {
        println!("current dir: {}", env::current_dir().unwrap().display());
        println!("open image: {}", path);

        let mut file = OpenOptions::new().write(true).read(true).open(path).unwrap();
        let lba_count = (file.seek(SeekFrom::End(0)).unwrap() / lba_size as u64) as u32;

        Self { 
            file: RefCell::new(file),
            lba_count,
            lba_size
        }
    }
}

impl BlockDevice for Image {
    type Error = bool;

    fn read(&self, lba: u32, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut file = self.file.borrow_mut();
        //println!("read: {:X}", lba as u64 * self.lba_size as u64 + offset as u64);
        file.seek(SeekFrom::Start(lba as u64 * self.lba_size as u64 + offset as u64)).unwrap();
        let len = file.read(buf).unwrap();
        Ok(len)
    }

    fn write(&self, lba: u32, offset: usize, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut file = self.file.borrow_mut();
        file.seek(SeekFrom::Start(lba as u64 * self.lba_size as u64 + offset as u64)).unwrap();
        let len = file.write(buf).unwrap();
        Ok(len)
    }

    fn flush(&self) -> Result<(), Self::Error> {
        let mut file = self.file.borrow_mut();
        file.flush().unwrap();
        Ok(())
    }

    fn count(&self) -> Result<u32, Self::Error> {
        Ok(self.lba_count)
    }

    fn lba_size(&self) -> Result<usize, Self::Error> {
        Ok(self.lba_size)
    }
}
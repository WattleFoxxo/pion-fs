extern crate pion_fs;
mod image;
use pion_fs::{FileSystem, Error, FatType};
use image::Image;
use pion_fs::dir::Dir;
use pion_fs::fs::Fs;
use pion_fs::file::File;
use pion_fs::block_device::BlockDevice;

fn print_tree<F: FileSystem>(dir: &Dir<F>, level: usize) -> Result<(), Error>{
    for entry in dir.iter() {
        let (dir_entry, lfn) = entry?;

        if dir_entry.compare(".") || dir_entry.compare("..") {
            continue;
        }

        if let Some(lfn) = lfn {
            for _ in 0..level {
                print!(" ");
            }

            lfn.print_name();
        }

        for _ in 0..level {
            print!(" ");
        }

        dir_entry.print_name();

        if dir_entry.is_dir() {
            let sub_dir = dir.open(&dir_entry)?;
            print_tree(&sub_dir, level + 1)?;
        }
    }
    Ok(())
}

fn log<'a, F: FileSystem>(dir: &Dir<'a, F>) -> Result<File<'a, F>, Error> {
    dir.create_file("HELL.LOG")
}

struct Volume<'a, D> {
    offset: u32,
    dev: &'a D,
}

impl <'a, D: BlockDevice> Volume<'a, D> {
    fn new(dev: &'a D, offset: u32) -> Self {
        Self {
            offset,
            dev,
        }
    }
}

impl <'a, D: BlockDevice >BlockDevice for Volume<'a, D> {
    type Error = D::Error;

    fn read(&self, lba: u32, offset: usize, buf: &mut [u8]) -> Result<usize, Self::Error> {
        self.dev.read(lba + self.offset, offset, buf)
    }

    fn write(&self, lba: u32, offset: usize, buf: &[u8]) -> Result<usize, Self::Error> {
        self.dev.write(lba + self.offset, offset, buf)
    }

    fn flush(&self) -> Result<(), Self::Error> {
        self.dev.flush()
    }

    fn count(&self) -> Result<u32, Self::Error> {
        self.dev.count()
    }

    fn lba_size(&self) -> Result<usize, Self::Error> {
        self.dev.lba_size()
    }
}

struct Drive<D> {
    drive: D,
}

impl <D: BlockDevice> Drive<D> {
    fn new(drive: D) -> Self {
        Self {
            drive,
        }
    }

    fn volume(&self, n: usize) -> Result<Volume<D>, pion_fs::Error> {
        Ok(Volume::new(&self.drive, 0))
    }
}

fn main() {
    //let image = Image::new("C:/xxx/hello/disk.img, 512");
    //let image = Image::new("C:/xxx/hello/0.img, 512");
    //let drive = Drive::new(Image::new("C:/xxx/hello/floppy.img, 512"));
    let drive = Drive::new(Image::new("examples/images/fat12_4k_sector_16MB.img", 4096));
    
    let volume = drive.volume(0).unwrap();
    let fs = Fs::mount(volume).unwrap();
    let root = Dir::root(&fs).unwrap();
    print_tree(&root, 0).unwrap();
}

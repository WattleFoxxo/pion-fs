use super::{FileSystem, Error, dir_entry};
use super::dir_entry::DirEntry;
use super::stream::Stream;
use super::file::File;
use super::path::Path;
use super::dir_iterator::DirIterator;
use super::lfn;

#[derive(Clone, Copy)]
pub struct Dir<'a, F> {
    fs: &'a F,
    cluster: u32,
}

impl <'a, F: FileSystem> Dir<'a, F> {
    pub fn root(fs: &'a F) -> Result<Self, Error> {
        Ok(Self {
            cluster: fs.root_cluster(),
            fs,
        })
    }

    pub fn open(&self, dir_entry: &DirEntry<'a, F>) -> Result<Self, Error> {
        if !dir_entry.is_dir() {
            return Err(Error::NotDir);
        }

        Ok(Self {
            cluster: dir_entry.cluster(),
            fs: self.fs,
        })
    }

    pub fn iter(&self) -> DirIterator<'a, F> {
        DirIterator::new(self.fs, self.cluster)
    }

    fn find_dir_entry(&self, name: &str) -> Result<DirEntry<'a, F>, Error> {
        for dir_entry in self.iter() {
            let (dir_entry, lfn) = dir_entry?;
            dir_entry.print_name();

            if dir_entry.compare(name) {
                return Ok(dir_entry);
            }

            if let Some(lfn) = lfn {
                lfn.print_name();

                if lfn.compare(name) {
                    return Ok(dir_entry);
                }
            }
        }

        Err(Error::NotFound)
    }

    fn follow(&self, path: &mut Path) -> Result<Dir<'a, F>, Error> {
        let mut dir = Dir {
            fs: self.fs,
            cluster: self.cluster,
        };
        
        for name in path {
            let dir_entry = dir.find_dir_entry(name)?;
            dir = dir.open(&dir_entry)?;
        }

        Ok(dir)
    }

    fn open_dir_entry(&self, path: &str) -> Result<DirEntry<'a, F>, Error> {
        let mut path = Path::new::<F>(path)?;
        let dir = self.follow(&mut path)?;
        dir.find_dir_entry(path.name())
    }

    fn create_dir_entry(&self, name: &str, is_file: bool, cluster: u32) -> Result<DirEntry<'a, F>, Error> {
        match self.find_dir_entry(name) {
            Ok(_) => return Err(Error::ObjectAlreadyExist),
            Err(Error::NotFound) => {},
            Err(e) => {
                return Err(e);
            }
        }

        let raw_dir_entry = dir_entry::create_raw(name, is_file, cluster)?;
        let lfn_size = lfn::lfn_need_space(name);
        let mut pos_to_write = DirIterator::find_free_space(self.fs, self.cluster, 1 + lfn_size)?;
        
        if lfn_size != 0 {
            let crc = dir_entry::checksum(&raw_dir_entry);
            lfn::lfn_serialize(self.fs, &mut pos_to_write, name, crc)?;
        }

        let dir_entry = DirEntry::from_raw_data(raw_dir_entry, self.fs, pos_to_write, None)?;
        dir_entry.flush()?;
        Ok(dir_entry)
    }

    pub fn create_dir(&self, path: &str) -> Result<Dir<'a, F>, Error> {
        let mut path = Path::new::<F>(path)?;
        let dir = self.follow(&mut path)?;
        let mut stream = Stream::create(self.fs)?;
        let raw_dir_entry = dir_entry::create_raw(".", false, stream.cluster())?;
        stream.write(self.fs, &raw_dir_entry)?;
        let raw_dir_entry = dir_entry::create_raw("..", false, dir.cluster)?;
        stream.write(self.fs, &raw_dir_entry)?;
        let dir_entry = dir.create_dir_entry(path.name(), false, stream.cluster())?;
        dir.open(&dir_entry)
    }

    pub fn create_file(&self, path: &str) -> Result<File<'a, F>, Error> {
        let mut path = Path::new::<F>(path)?;
        let dir = self.follow(&mut path)?;
        let stream = Stream::create(self.fs)?;
        let dir_entry = dir.create_dir_entry(path.name(), true, stream.cluster())?;
        File::open(dir_entry)
    }

    pub fn open_file(&self, path: &str) -> Result<File<'a, F>, Error> {
        File::open(self.open_dir_entry(path)?)
    }

    pub fn open_dir(&self, path: &str) -> Result<Dir<'a, F>, Error> {
        self.open(&self.open_dir_entry(path)?)
    }

    pub fn remove_dir(&self, path: &str) -> Result<(), Error> {
        let dir_entry = self.open_dir_entry(path)?;

        if !dir_entry.is_dir() {
            return Err(Error::NotDir);
        }

        let dir = self.open(&dir_entry)?;

        if dir.item_count()? != 0 {
            return Err(Error::DirNotEmpty);
        }

        Stream::remove(self.fs, dir_entry.cluster())?;
        dir_entry.remove()
    }

    pub fn remove_file(&self, path: &str) -> Result<(), Error> {
        let dir_entry = self.open_dir_entry(path)?;
        
        if !dir_entry.is_file() {
            return Err(Error::NotFile);
        }

        Stream::remove(self.fs, dir_entry.cluster())?;
        dir_entry.remove()
    }

    pub fn item_count(&self) -> Result<usize, Error> {
        let mut count = 0;

        for entry in self.iter() {
            let (dir_entry, _) = entry?;

            if dir_entry.compare(".") || dir_entry.compare("..") {
                continue;
            }

            count += 1;
        }

        Ok(count)
    }
}

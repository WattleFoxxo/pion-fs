use super::{FileSystem, Error};
use super::stream::Stream;

pub const DIR_ENTRY_SIZE: usize = 32;

pub const ATTR_READ_ONLY: u8 = 0x01;
pub const ATTR_HIDDEN: u8 = 0x02;
pub const ATTR_SYSTEM: u8 = 0x04;
pub const ATTR_VOLUME_ID: u8 = 0x08;
pub const ATTR_DIRECTORY: u8 = 0x10;
pub const ATTR_ARCHIVE: u8 = 0x20;

pub const REMOVED_ENTRY: u8 = 0xE5;
pub const FREE_ENTRY: u8 = 0x00;

pub struct DirEntry<'a, F> {
    fs: &'a F,
    raw: [u8; DIR_ENTRY_SIZE],
    stream: Stream,
    lfn_pos: Option<Stream>, // lfn location, if exist
}

pub fn create_raw(name: &str, is_file: bool, cluster: u32) -> Result<[u8; DIR_ENTRY_SIZE], Error> {
    let mut raw = [
       0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, // name 
       0x00, // attr
       0x00, // nt res
       0x00, // create time tenth
       0xB6, 0x3C, // create time
       0x4D, 0x46, // create date
       0x4D, 0x46, // acc date 
       (cluster >> 16) as u8, (cluster >> 24) as u8, // clust high
       0xB6, 0x3C, // write time
       0x4D, 0x46, // write date
       cluster as u8, (cluster >> 8) as u8, // clust low
       0x00, 0x00, 0x00, 0x00, // file size
   ];

   if !is_file {
       raw[11] = ATTR_DIRECTORY;
   }

   if is_file {
       let (name, ext) = split_name_end_ext(name);
       // copy name
       let mut i = 0;
       
       for c in name.chars() {
            if c as u8 >= b'a' && c as u8 <= b'z' {
                raw[i] = c as u8 - b'a' + b'A';
            } else {
                raw[i] = c as u8;
            }
           i += 1;

           if i == 8 {
               break;
           }
       }
       // copy extension
       if let Some(ext) = ext {
           let mut i = 0;
       
           for c in ext.chars() {
            if c as u8 >= b'a' && c as u8 <= b'z' {
                raw[8 + i] = c as u8 - b'a' + b'A';
            } else {
                raw[8 + i] = c as u8;
            }
               i += 1;

               if i == 3 {
                   break;
               }
           }
       }
    } else {
        let mut i = 0;
       
        for c in name.chars() {
            if c as u8 >= b'a' && c as u8 <= b'z' {
                raw[i] = c as u8 - b'a' + b'A';
            } else {
                raw[i] = c as u8;
            }

            i += 1;

           if i == 8 {
               break;
           }
       }
   }

   Ok(raw)
}

impl <'a, F: FileSystem> DirEntry<'a, F> {
    pub fn from_raw_data(data: [u8; DIR_ENTRY_SIZE], fs: &'a F, stream: Stream, lfn: Option<(Stream, u8)>) -> Result<Self, Error> {
        let mut lfn_pos = None;

        if let Some((pos, crc)) = lfn {
            if checksum(&data) == crc {
                lfn_pos = Some(pos);
            }
        }

        Ok(Self {
            raw: data,
            stream,
            fs,
            lfn_pos,
        })
    }

    pub fn is_file(&self) -> bool {
        (self.raw[11] & (ATTR_VOLUME_ID | ATTR_DIRECTORY)) == 0x00
    }

    pub fn is_dir(&self) -> bool {
        (self.raw[11] & (ATTR_VOLUME_ID | ATTR_DIRECTORY)) == ATTR_DIRECTORY
    }

    pub fn size(&self) -> u32 {
        u32::from_le_bytes([self.raw[28], self.raw[29], self.raw[30], self.raw[31]])
    }

    pub fn set_size(&mut self, size: u32) {
        self.raw[28] = size as u8;
        self.raw[29] = (size >> 8) as u8;
        self.raw[30] = (size >> 16) as u8;
        self.raw[31] = (size >> 24) as u8;
    }

    pub fn cluster(&self) -> u32 {
        let cluster_h = u16::from_le_bytes([self.raw[20], self.raw[21]]);
        let cluster_l = u16::from_le_bytes([self.raw[26], self.raw[27]]);
        ((cluster_h as u32) << 16) | (cluster_l as u32)
    }

    pub fn flush(&self) -> Result<(), Error> {
        let mut stream = self.stream;
        stream.write(self.fs, &self.raw)?;
        Ok(())
    }

    pub fn compare(&self, name: &str) -> bool {
        let (buf, len) = self.name();
        
        if name.len() != len {
            return false;
        }

        for (i, &c) in name.as_bytes().into_iter().enumerate() {
            if buf[i] != c {
                return false;
            }
        }

        return true;
    }

    fn name(&self) -> ([u8; 12], usize) {
        let mut buf = [0u8; 12];
        let mut len = 0;

        for &c in &self.raw[..8] {
            if c == b' ' {
                break;
            }

            buf[len] = c;
            len += 1;
        }

        if self.is_file() && self.raw[8] != b' ' {
            buf[len] = b'.';
            len += 1;
        }

        for &c in &self.raw[8..11] {
            if c == b' ' {
                break;
            }

            buf[len] = c;
            len += 1;
        }

        (buf, len)
    }

    pub fn print_name(&self) {
        let (buf, len) = self.name();

        for &c in &buf[..len] {
            print!("{}", c as char);
        }

        println!();
    }

    pub fn remove(mut self) -> Result<(), Error> {
        if let Some(mut lfn_pos) = self.lfn_pos {
            // remove LFN
            let mut buf = [0u8; DIR_ENTRY_SIZE];
            buf[0] = REMOVED_ENTRY;

            while !lfn_pos.is_equal(&self.stream) {
                lfn_pos.write(self.fs, &buf)?;
            }
        }

        self.raw[0] = REMOVED_ENTRY;
        self.flush()
    }

    pub fn fs(&self) -> &'a F {
        self.fs
    }
}

pub fn checksum(buf: &[u8]) -> u8 {
    let mut checksum = 0;

    for i in 0..11 {
        // NOTE: The operation is an unsigned char rotate right
        if checksum & 1 != 0 {
            checksum = buf[i].wrapping_add(0x80 + (checksum >> 1));
        } else {
            checksum = buf[i].wrapping_add(checksum >> 1);
        }
    }
    
    checksum
}

fn split_name_end_ext(name: &str) -> (&str, Option<&str>) {
    for (i, c) in name.chars().rev().enumerate() {
        if c == '.' {
            return (&name[..name.len() - i - 1], Some(&name[(name.len() - i)..]));
        }
    }

    return (name, None);
}

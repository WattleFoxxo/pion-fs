use super::dir_entry::DIR_ENTRY_SIZE;
use super::dir_entry::{ATTR_ARCHIVE, ATTR_DIRECTORY, ATTR_HIDDEN, ATTR_READ_ONLY, ATTR_VOLUME_ID, ATTR_SYSTEM};
use super::stream::Stream;
use super::{FileSystem, Error};

const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;
const ATTR_LONG_NAME_MASK: u8 = ATTR_LONG_NAME | ATTR_DIRECTORY | ATTR_ARCHIVE;

pub const LFN_MAX_LEN: usize = 256;
const LAST_LONG_ENTRY: u8 = 0x40;
const LAST_LONG_ENTRY_MASK: u8 = 0xf0;

const CHAR_ORDER: [usize; 13] = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];

pub struct Lfn {
    buf: [u8; LFN_MAX_LEN],
    len: usize,
}

impl Lfn {
    pub fn new() -> Self {
        Self {
            buf: [0u8; LFN_MAX_LEN],
            len: 0,
        }
    }

    pub fn compare(&self, name: &str) -> bool {
        println!("lfn name len: {}, compared name len: {}", self.len, name.len());

        if name.len() != self.len {
            return false;
        }

        for (i, &c) in name.as_bytes().iter().enumerate() {
            if self.buf[i] != c {
                return false;
            }
        }

        true
    }

    pub fn name(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    pub fn print_name(&self) {
        for &c in &self.buf[..self.len] {
            print!("{}", c as char);
        }

        println!();
    }
}

pub struct LfnBuilder<'a> {
    lfn: &'a mut Lfn,
    lfn_pos: Stream,
    checksum: u8,
    number: u8,
}

impl <'a> LfnBuilder<'a> {
    pub fn new(lfn: &'a mut Lfn) -> Self {
        Self { 
            lfn,
            lfn_pos: Stream::open(0),
            checksum: 0,
            number: 0,
        }
    }

    pub fn reset(&mut self) {
        self.number = 0;
    }

    pub fn process(&mut self, pos: Stream, buf: &[u8; DIR_ENTRY_SIZE]) -> bool {
        if buf[11] & ATTR_LONG_NAME_MASK != ATTR_LONG_NAME {
            // not LFN
            return false;
        }

        if buf[0] & LAST_LONG_ENTRY_MASK == LAST_LONG_ENTRY {
            // begining of LFN
            self.checksum = buf[13];
            self.number = buf[0] & 0x0f;
            self.lfn_pos = pos;
            self.lfn.len = self.number as usize * CHAR_ORDER.len();
        } else {
            if buf[13] != self.checksum {
                // wrong LFN checksum
                return false;
            }

            let number = buf[0] & 0x0f;

            if number + 1 != self.number {
                // wrong LFN order
                return false;
            }

            self.number = number;
        }

        if self.number == 0 {
            // invalid number
            return false;
        }

        let start = (self.number as usize - 1) * CHAR_ORDER.len();

        for (i, &n) in CHAR_ORDER.iter().enumerate() {
            if buf[n] == 0x00 {
                self.lfn.len = start + i;
                break;
            }

            self.lfn.buf[start + i] = buf[n];
        }
        
        return true;
    }

    pub fn build(&mut self) -> Option<(Stream, u8)> {
        if self.number == 1 {
            Some((self.lfn_pos, self.checksum))
        } else {
            None
        }
    }
}

pub fn lfn_need_space(name: &str) -> usize {
    let count = (name.len() + 1) / CHAR_ORDER.len();

    if (name.len() + 1) % CHAR_ORDER.len() != 0 {
        return count + 1;
    }

    count
}

pub fn lfn_serialize<'a, F: FileSystem>(fs: &F, pos_to_write: &mut Stream, name: &str, crc: u8) -> Result<(), Error> {
    let count = lfn_need_space(name);

    for i in (0..count).rev() {
        let mut dir_entry = [0u8; 32];
        
        if i == count - 1 {
            dir_entry[0] = LAST_LONG_ENTRY;
        }

        dir_entry[0] |= i as u8 + 1;
        dir_entry[11] = ATTR_LONG_NAME;
        dir_entry[13] = crc;

        let start = i * CHAR_ORDER.len();
        let range = core::cmp::min(name.len() - start, CHAR_ORDER.len());
        let end = start + range;

        for (i, c) in name[start..end].chars().enumerate() {
            dir_entry[CHAR_ORDER[i]] = c as u8;
        }

//        if range != CHAR_ORDER.len() {
//        
//        }
        pos_to_write.write(fs, &dir_entry)?;
    }

    Ok(())
}
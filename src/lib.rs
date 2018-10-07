use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::io;
use std::io::ErrorKind;
use std::io::SeekFrom;

pub struct Termite {
    inode: u64,
    trim_pos: usize,
    reader: BufReader<File>,
}

impl Termite {
    pub fn new(file: File, inode: u64) -> io::Result<Termite> {
        let reader = BufReader::new(file);
        Ok(Termite{inode: inode, trim_pos: 0, reader: reader})
    }

    pub fn chew<F>(&mut self, mut apply: F) -> Result<(), std::io::Error> where
        F: FnMut(&str) -> Result<usize, std::io::Error> {
        let mut buf = String::new();
        while let Ok(read_count) = self.reader.read_line(&mut buf) {
            if read_count == 0 { break; }
            let buf_clone = buf.clone();
            let clean_buf = buf_clone.trim_right();
            apply(clean_buf)?;
            buf.truncate(0);
            self.reader.get_mut().seek(SeekFrom::Current(read_count as i64))?;
            self.trim_pos = self.trim_pos + read_count;
        }
        if buf.len() == 0 {
            Ok(())
        } else {
            Err(std::io::Error::new(ErrorKind::Other, "unknown error reading line".to_string()))
        }
    }
}

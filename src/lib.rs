use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::SeekFrom;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

extern crate libc;
use libc::{fallocate, FALLOC_FL_PUNCH_HOLE, FALLOC_FL_KEEP_SIZE};

pub struct Termite {
    fd: RawFd,
    blksize: u64,
    trim_pos: usize,
    reader: BufReader<File>,
}

impl Termite {
    pub fn new(file: File, blocksize: u64) -> io::Result<Termite> {
        let fd = file.as_raw_fd();
        let reader = BufReader::new(file);
        Ok(Termite{fd: fd, blksize: blocksize, trim_pos: 0, reader: reader})
    }

    pub fn chew<F>(&mut self, mut apply: F) -> Result<(), std::io::Error> where
        F: FnMut(&str) -> Result<usize, std::io::Error> {
        let mut buf = String::new();
        loop {
            let read_count = self.reader.read_line(&mut buf)?;
            if read_count == 0 { break; }
            let buf_clone = buf.clone();
            let clean_buf = buf_clone.trim_right();
            apply(clean_buf)?;
            buf.truncate(0);
            self.reader.get_mut().seek(SeekFrom::Current(read_count as i64))?;
            self.trim_pos = self.trim_pos + read_count;
            self.physical_trim();
        }
        if buf.len() == 0 {
            Ok(())
        } else {
            Err(std::io::Error::new(ErrorKind::Other, "unknown error reading line".to_string()))
        }
    }

    fn physical_trim(&mut self) -> () {
        if self.trim_pos as u64 > self.blksize {
            println!("trimmy");
            unsafe {
                let fresh = fallocate(self.fd, FALLOC_FL_PUNCH_HOLE|FALLOC_FL_KEEP_SIZE, 0, self.trim_pos as i64);
                if fresh < 0 {
                    println!("fd {} fresh {} errno {}", self.fd, fresh, Error::last_os_error());
                }
            }
            self.trim_pos = 0;
        }
    }
}

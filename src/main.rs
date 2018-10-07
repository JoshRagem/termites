use std::collections::HashMap;
use std::env;
use std::env::args;
use std::fs::{File, Metadata};
use std::net::{UdpSocket, SocketAddr};
use std::os::unix::fs::MetadataExt;
use std::process::exit;

extern crate time;

extern crate termites;
use termites::Termite;

fn get_params() -> Option<(SocketAddr, SocketAddr, String, String)> {
    let mut params = args();
    params.next();
    let server = params.next().map(|s: String| s.parse::<SocketAddr>().ok());
    let local = params.next().map(|s: String| s.parse::<SocketAddr>().ok());
    let host = params.next();
    let filename = params.next();
    if let (Some(Some(s)), Some(Some(l)), Some(h), Some(f)) = (server, local, host, filename) {
	Some((s, l, f, h))
    } else {
        None
    }
}

fn main(){
    let (syslog_addr, local_addr, filename, hostname) = match get_params() {
        Some(x) => x,
        None => {
            eprintln!("usage: termites server:port local:port hostname filename");
            exit(1);
        }
    };

    let proc_name = env::var("SERVICE_NAME").ok().unwrap_or(hostname.clone());

    let mut formatter = TermiteFormatter {
        facility: 16*8,
        hostname: hostname,
        process: proc_name,
        pid: 0,
        message_count: 0,
    };

    let mut udp_socket = match bind_socket(local_addr) {
	Some(u) => u,
	None => {
	    eprintln!("unable to bind addr {}", local_addr);
	    exit(2);
        }
    };

    let (file, meta) = match file_data(filename) {
        Ok((f, m)) => (f, m),
        Err(e) => {
            eprintln!("error opening file: {}", e);
            exit(3);
        }
    };

    let mut termite = match Termite::new(file, meta.ino()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("unknown error reading file: {}", e);
            exit(4);
        }
    };

    let default_data: StructuredData = HashMap::new();
    match termite.chew(|line: &str|
                 formatter.format(&mut udp_socket, syslog_addr, 6, TermiteLog{data: &default_data, message: line.to_string()})
                 )
    {
        Ok(_) => println!("eof reached"),
        Err(e) => {
            eprintln!("unknown error reading file: {}", e);
            exit(5);
        }
    };

}

type StructuredData = HashMap<String, HashMap<String, String>>;
struct TermiteLog<'a> {
    data: &'a StructuredData,
    message: String
}
impl<'a> std::fmt::Display for TermiteLog<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Clone,Debug)]
struct TermiteFormatter {
    facility: u8,
    hostname: String,
    process:  String,
    pid:      i32,
    message_count: i32,
}
impl TermiteFormatter {
    fn format_5424_structured_data(&self, data: &StructuredData) -> String {
        if data.is_empty() {
            "-".to_string()
        } else {
            let mut res = String::new();
            for (id, params) in data {
                res = res + "["+id;
                for (name,value) in params {
                    res = res + " " + name + "=\"" + value + "\"";
                }
                res += "]";
            }
            res
        }
    }
    pub fn format(&mut self, sock: &mut UdpSocket, addr: SocketAddr, severity: u8, log: TermiteLog) -> Result<usize, std::io::Error> {
        let message_id = self.message_count;
        self.message_count+=1;
        let syslog = format!(
            "<{}> {} {} {} {} {} {} {} {}",
            self.facility | severity,
            1, // version
            time::now_utc().rfc3339(),
            self.hostname,
            self.process, self.pid, message_id,
            self.format_5424_structured_data(log.data), log.message
            );
        println!("syslog: {}", syslog);
        sock.send_to(syslog.as_bytes(), addr)
  }
}

fn bind_socket(sock: SocketAddr) -> Option<UdpSocket> {
    UdpSocket::bind(sock).ok()
}

fn file_data(filename: String) -> std::io::Result<(File, Metadata)> {
    let file = File::open(filename)?;
    let meta = file.metadata()?;
    Ok((file, meta))
}

use std::os::unix::net::UnixStream;
use common::SOCKET_PATH;
use std::io::Write;

fn main() {
    let mut unix_stream =
        UnixStream::connect(SOCKET_PATH).expect("Could not create stream");
    let c = std::env::args().nth(1).unwrap();
    unix_stream.write(c.as_bytes()).expect("unable to write to socket");
}

use ctru::prelude::*;
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::time::Duration;

pub const BOTTOM_WIDTH: i32 = 30;
pub const BOTTOM_HEIGHT: i32 = 20;

pub const TOP_WIDTH: i32 = 40;
pub const TOP_HEIGHT: i32 = 30;

static DSX_DIR: &str = "sdmc:/3ds/";

fn main() {
    let gfx = Gfx::new().unwrap();
    let mut hid = Hid::new().unwrap();
    let apt = Apt::new().unwrap();
    let soc = Soc::new().unwrap();

    let server = TcpListener::bind("0.0.0.0:8080").unwrap();
    server.set_nonblocking(true).unwrap();

    let _bottom_console = Console::new(gfx.bottom_screen.borrow_mut());

    println!("Listening on host address: {}\n", soc.host_address());
    println!("\x1b[29;12HPress Start to exit");

    let _top_console = Console::new(gfx.top_screen.borrow_mut());

    let mut response: String =
        String::from("default response...something is wrong.\nshould have returned a status");

    while apt.main_loop() {
        hid.scan_input();
        if hid.keys_down().contains(KeyPad::START) {
            break;
        };

        match server.accept() {
            Ok((mut stream, socket_addr)) => {
                println!("Got connection from {socket_addr}");

                let mut buf = [0u8; 5242880]; // 5 mb
                let bytes_read = match stream.read(&mut buf) {
                    Ok(num_bytes) => {
                        let mut title: Vec<u8> = Vec::new();
                        let mut body: Vec<u8> = Vec::new();

                        let parts = match split_at_newline(&buf[..num_bytes]) {
                            Some((a, b)) => {
                                title.extend_from_slice(a);
                                body.extend_from_slice(b);
                                (a, b)
                            }
                            None => {
                                println!("FATAL; NO TITLE SPECIFIED IN BYTE BUFFER");
                                std::process::exit(1);
                            }
                        };

                        let title = String::from_utf8_lossy(parts.0).into_owned();

                        let mut f = match std::fs::File::create(title.clone()) {
                            Ok(file) => file,
                            Err(e) => {
                                println!("Failed to create file: {e}");
                                response = format!("Failed to create file because of error: {}", e);
                                continue;
                            }
                        };

                        match f.write_all(parts.1) {
                            Ok(_) => {
                                println!(
                                    "Successfully wrote {} bytes from buffer to file: {}",
                                    num_bytes, title
                                );
                                response = format!(
                                    "Successfully wrote {} bytes to file {}",
                                    num_bytes, title
                                );
                            }
                            Err(e) => {
                                println!("FAILED TO WRITE BUFFER TO FILE: {e}");
                                response = format!(
                                    "Failed to write buffer to file because of error: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            println!("Note: Reading the connection returned ErrorKind::WouldBlock. Trying again.");
                        } else {
                            println!("Unable to read stream: {e}");
                        }
                    }
                };
                let response_bytes = response.as_bytes();

                if let Err(e) = stream.write(response_bytes) {
                    println!("Error writing http response: {e}");
                }
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {}
                _ => {
                    println!("Error accepting connection: {e}");
                    std::thread::sleep(Duration::from_secs(2));
                }
            },
        }

        gfx.wait_for_vblank();
    }
}

fn split_at_newline(data: &[u8]) -> Option<(&[u8], &[u8])> {
    if let Some(pos) = data.iter().position(|&c| c == b'\n') {
        Some((&data[..pos], &data[pos + 1..]))
    } else {
        None
    }
}

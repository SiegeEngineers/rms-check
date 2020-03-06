//! Hacky, yes :(

use rms_check_lsp::RMSCheckLSP;
use std::io::{self, BufRead, Write};

/// More or less copied from RLS:
/// https://github.com/rust-lang/rls/blob/36def189c0ef802b7ca07878100c856f492532cb/rls/src/server/io.rs
fn read_message<R: BufRead>(from: &mut R) -> io::Result<String> {
    let mut length = None;

    loop {
        let mut line = String::new();
        from.read_line(&mut line)?;
        if line == "\r\n" {
            break;
        }

        let parts: Vec<&str> = line.split(": ").collect();
        if parts.len() != 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Header '{}' is malformed", line),
            ));
        }
        let header_name = parts[0].to_lowercase();
        let header_value = parts[1].trim();
        match header_name.as_ref() {
            "content-length" => {
                length = Some(usize::from_str_radix(header_value, 10).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "content-length is not a number")
                })?)
            }
            "content-type" => (),
            _ => (),
        }
    }

    let length = length.unwrap();
    let mut message = vec![0; length];
    from.read_exact(&mut message)?;
    String::from_utf8(message).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_message(message: &str) {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        message.len(),
        message
    )
    .unwrap();
    writer.flush().unwrap();
}

/// Start the language server.
pub fn cli_server() {
    let mut lsp = RMSCheckLSP::new(|message| {
        let message = serde_json::to_string(&message).unwrap();
        write_message(&message);
    });

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    loop {
        let message = read_message(&mut reader).expect("could not read message");

        if let Some(response) = lsp.handle_sync(message.parse().unwrap()) {
            let response = serde_json::to_string(&response).unwrap();
            write_message(&response);
        }
    }
}

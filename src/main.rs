use std::{
    cmp::min,
    fs::File,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
};

// PROTO/1.0\n              <-- Protocol version line
// Command: UPLOAD\n        <-- The action command
// Filename: <filename>\n   <-- The name of the file
// Filesize: <filesize>\n   <-- The size of the file data in bytes
// \n                       <-- Empty line signifies the end of the header
//
// RESPONSE/1.0\n      <-- Response protocol version
// Status: OK\n        <-- The result status
// \n                  <-- Empty line signifies the end of the response header

const UPLOADS_DIR: &str = "uploads";
const PROTOCOL_VERSION: &str = "PROTO/1.0"; // Define client protocol version
const RESPONSE_VERSION: &str = "RESPONSE/1.0"; // Define server response version

#[derive(Debug)]
enum Command {
    Upload,
    Download,
}

fn handle_client(mut stream: TcpStream) {
    // This function will handle communication with a single client
    let mut reader = BufReader::new(&mut stream);
    let mut header_lines: Vec<String> = Vec::new();
    println!("Connection established!");
    for _ in 0..4 {
        let mut line = String::new();

        match reader.read_line(&mut line) {
            Ok(0) => {
                return send_error(
                    &mut stream,
                    "Connection closed prematurely during header read",
                );
            }

            Ok(_) => {
                // \n\n
                if line.trim().is_empty() {
                    break;
                }

                header_lines.push(line);

                if header_lines.len() == 4 {
                    break;
                }
            }

            Err(e) => {
                return send_error(
                    &mut stream,
                    format!("Error reading header line, {}", e).as_str(),
                );
            }
        }
    }
    println!(
        "Received header lines: {:?}",
        header_lines.iter().map(|s| s.trim()).collect::<Vec<_>>()
    );

    if header_lines.is_empty() || header_lines[0].trim() != PROTOCOL_VERSION {
        send_error(&mut stream, "Invalid or missing PROTOCOL version");
        return;
    }

    let command = match header_lines.get(1).map(|s| s.trim()) {
        Some("COMMAND: UPLOAD") => Some(Command::Upload),
        Some("COMMAND: DOWNLOAD") => Some(Command::Download),
        _ => {
            return send_error(
                &mut stream,
                "Invalid Command, `COMMAND: UPLOAD` and `COMMAND: DOWNLAOD` supported",
            );
        }
    };

    let Some(filename_line) = header_lines.get(2).map(|s| s.trim()) else {
        return send_error(&mut stream, "File name not attached");
    };

    let Some(filename) = filename_line.strip_prefix("FILENAME: ") else {
        return send_error(&mut stream, "Invalid filename");
    };

    let Some(filesize_line) = header_lines.get(3).map(|s| s.trim()) else {
        return send_error(&mut stream, "File size not attached");
    };
    let mut filesize: usize = 0;

    if let Some(fs_str) = filesize_line.strip_prefix("FILESIZE: ") {
        match fs_str.trim().parse::<usize>() {
            Ok(size) => {
                filesize = size; // Parse successful, store the size
            }
            Err(_) => {
                // Had the header and prefix, but the value wasn't a valid number
                // This is always an error if the header is present.
                return send_error(&mut stream, "Invalid FILESIZE value: not a valid number");
            }
        }
    } else {
        if let Some(Command::Upload) = command {
            // This is an error only for UPLOAD command
            return send_error(
                &mut stream,
                "Invalid FILESIZE header format for UPLOAD. Expected 'FILESIZE: <size>'",
            );
        }
    };

    match command {
        Some(Command::Upload) => {
            if filesize == 0 {
                return send_error(&mut stream, "something went wrong with file size");
            }
            match receive_and_save_file(&mut stream, filename, filesize) {
                Ok(_) => {
                    println!("File '{}' uploaded successfully.", filename);
                    // Send success response header using the stream returned by into_inner()
                    let response = format!("{}\nStatus: OK\n\n", RESPONSE_VERSION);
                    if let Err(e) = stream.write_all(response.as_bytes()) {
                        eprintln!("Error sending OK response: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("ERROR uploading file '{}': {}", filename, e);
                    // Send error response header with message using the stream returned by into_inner()
                    let error_msg =
                        format!("{}Status: ERROR\nMessage: {}\n\n", RESPONSE_VERSION, e);
                    // Corrected typo here: stream_for_all -> stream_for_file
                    if let Err(e) = stream.write_all(error_msg.as_bytes()) {
                        eprintln!("Error sending ERROR response: {}", e);
                    }
                }
            }
        }
        _ => {}
    }

    println!("Connection from {} closed.", stream.peer_addr().unwrap_or_else(|_| "unknown address".parse().unwrap()));

}

fn main() -> std::io::Result<()>  {
    let listener = TcpListener::bind("127.0.0.1:6969").unwrap();

    // Get the local address the listener is bound to and print it
    let local_addr = listener.local_addr()?;
    println!("Server listening on {}", local_addr);

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                handle_client(_stream);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

fn receive_and_save_file(
    stream: &mut TcpStream,
    filename: &str,
    filesize: usize,
) -> std::io::Result<()> {
    if filename.contains("..") || filename.starts_with("/") || filename.contains("\\") {
        eprintln!(
            "Security Alert: Attempted path traversal with filename: {}",
            filename
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid filename specified (potential path traversal)",
        ));
    }

    std::fs::create_dir_all(UPLOADS_DIR)?;
    println!("Ensured uploads directory exists: {}", UPLOADS_DIR);
    let file_path = Path::new(UPLOADS_DIR).join(filename);

    println!("Attempting to create file: {:?}", file_path);
    let mut file = match File::create(&file_path) {
        Ok(f) => {
            println!("Successfully created file: {:?}", file_path);
            f
        }
        Err(e) => {
            eprintln!("Error creating file {:?}: {}", file_path, e);
            return Err(e);
        }
    };

    // Read exactly `filesize` bytes from the stream
    let mut received_bytes = 0;
    // Use a reasonably sized buffer for reading chunks
    let mut buffer = vec![0; 4096]; // 4KB buffer

    println!(
        "Starting to receive file data (expecting {} bytes)...",
        filesize
    );

    while received_bytes < filesize {
        // Calculate how many bytes we still need
        let bytes_remaining = filesize - received_bytes;
        // Determine the maximum number of bytes to read in this iteration
        let bytes_to_read = min(buffer.len(), bytes_remaining);

        // Read data into the buffer slice
        let bytes_read = stream.read(&mut buffer[..bytes_to_read])?;

        // Check if the connection was closed before we received all data
        if bytes_read == 0 {
            eprintln!(
                "Connection closed prematurely. Expected {} bytes, received {}.",
                filesize, received_bytes
            );
            // Clean up the partially created file? Or leave it? For now, just return error.
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "Connection closed before receiving expected {} bytes. Received only {}.",
                    filesize, received_bytes
                ),
            ));
        }

        // Write the received chunk to the file
        file.write_all(&buffer[..bytes_read])?;

        received_bytes += bytes_read;
        // Optional: print progress
        println!("Received {}/{} bytes...", received_bytes, filesize);
    }

    // Ensure all buffered data is written to the file
    file.flush()?;
    Ok(())
}

fn send_error(stream: &mut TcpStream, message: &str) {
    eprintln!("ERROR: {}", message);
    let response = format!(
        "{}Status: ERROR\nMessage: {}\n\n",
        RESPONSE_VERSION, message
    );
    let _ = stream.write_all(response.as_bytes());
}

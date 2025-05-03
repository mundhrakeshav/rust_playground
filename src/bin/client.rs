use std::{
    env, // For command-line arguments
    fs::File,
    io::{self, BufRead, BufReader, Read, Write}, // Input/output operations
    net::TcpStream, // TCP stream for network communication
    path::Path,     // Path manipulation
    process,        // For exiting the process
    time::Duration, // For setting timeouts
};

// --- Configuration ---
const SERVER_ADDR: &str = "127.0.0.1:6969"; // Server address and port
const PROTOCOL_VERSION: &str = "PROTO/1.0"; // Client protocol version
const RESPONSE_VERSION: &str = "RESPONSE/1.0"; // Expected response protocol version
const BUFFER_SIZE: usize = 4096; // Size of chunks for sending file data
const TIMEOUT_SECS: u64 = 10; // Connection and read/write timeout in seconds

fn main() -> io::Result<()> {
    // --- 1. Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <filepath>", args[0]);
        process::exit(1); // Exit with error code
    }
    let filepath_str = &args[1];
    let filepath = Path::new(filepath_str);

    // --- 2. Validate File and Get Metadata ---
    if !filepath.is_file() {
        eprintln!("Error: File not found or is not a regular file at '{}'", filepath_str);
        process::exit(1);
    }

    let filename = match filepath.file_name() {
        Some(name) => name.to_str().unwrap_or_else(|| {
            eprintln!("Error: Invalid filename (non-UTF8).");
            process::exit(1);
        }),
        None => {
            eprintln!("Error: Could not extract filename from path '{}'", filepath_str);
            process::exit(1);
        }
    };

    let file_metadata = match std::fs::metadata(filepath) {
        Ok(meta) => meta,
        Err(e) => {
            eprintln!("Error accessing file metadata for '{}': {}", filepath_str, e);
            process::exit(1);
        }
    };
    let filesize = file_metadata.len(); // Get file size in bytes

    println!("Preparing to upload '{}' ({} bytes)", filename, filesize);

    // --- 3. Construct the Header ---
    let header = format!(
        "{}\nCOMMAND: UPLOAD\nFILENAME: {}\nFILESIZE: {}\n\n", // Double \n signifies end
        PROTOCOL_VERSION, filename, filesize
    );

    // --- 4. Establish TCP Connection ---
    println!("Connecting to {}...", SERVER_ADDR);
    let mut stream = match TcpStream::connect(SERVER_ADDR) {
        Ok(s) => {
            println!("Connected.");
            s
        }
        Err(e) => {
            eprintln!("Error connecting to server {}: {}", SERVER_ADDR, e);
            process::exit(1);
        }
    };

    // Set read and write timeouts for the stream
    let timeout_duration = Duration::from_secs(TIMEOUT_SECS);
    if let Err(e) = stream.set_read_timeout(Some(timeout_duration)) {
         eprintln!("Warning: Failed to set read timeout: {}", e);
    }
    if let Err(e) = stream.set_write_timeout(Some(timeout_duration)) {
         eprintln!("Warning: Failed to set write timeout: {}", e);
    }


    // --- 5. Send Header and File Data ---
    if let Err(e) = send_data(&mut stream, &header, filepath, filesize) {
        eprintln!("Error during sending data: {}", e);
        // Attempt to close stream gracefully even on error
        let _ = stream.shutdown(std::net::Shutdown::Both);
        process::exit(1);
    }

    // --- 6. Receive and Print Server Response ---
    println!("Waiting for server response...");
    if let Err(e) = receive_response(&mut stream) {
         eprintln!("Error receiving or processing server response: {}", e);
         // Attempt to close stream gracefully even on error
         let _ = stream.shutdown(std::net::Shutdown::Both);
         process::exit(1);
    }

    // --- 7. Close Connection (implicitly done when stream goes out of scope) ---
    println!("Closing connection.");
    // Optionally, explicitly shut down write side:
    // stream.shutdown(std::net::Shutdown::Write)?;

    Ok(()) // Indicate successful execution
}

// --- Helper function to send header and file ---
fn send_data(
    stream: &mut TcpStream,
    header: &str,
    filepath: &Path,
    filesize: u64,
) -> io::Result<()> {
    // Send the header
    println!("Sending header...");
    stream.write_all(header.as_bytes())?;
    println!("Header sent.");

    // Send the file content
    println!("Sending file data...");
    let mut file = File::open(filepath)?;
    let mut buffer = vec![0; BUFFER_SIZE];
    let mut bytes_sent: u64 = 0;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // End of file
        }
        stream.write_all(&buffer[..bytes_read])?;
        bytes_sent += bytes_read as u64;
        // Optional: print progress
        println!("Sent {}/{} bytes...", bytes_sent, filesize);
    }

    // Ensure all buffered data is sent
    stream.flush()?;

    println!("File data sent ({} bytes).", bytes_sent);

    if bytes_sent != filesize {
         eprintln!(
            "Warning: Sent {} bytes, but expected file size was {}.",
             bytes_sent, filesize
         );
         // Decide if this should be a hard error depending on requirements
         // return Err(io::Error::new(io::ErrorKind::Other, "File size mismatch during send"));
    }

    Ok(())
}

// --- Helper function to receive and parse response ---
fn receive_response(stream: &mut TcpStream) -> io::Result<()> {
    // Use BufReader for efficient line-based reading
    let mut reader = BufReader::new(stream);
    let mut response_header_lines: Vec<String> = Vec::new();
    let mut lines_read = 0;

    loop {
        let mut line = String::new();
        // Read one line, including the newline character
        match reader.read_line(&mut line) {
            Ok(0) => {
                // Connection closed prematurely
                eprintln!("Warning: Connection closed by server before receiving full response header.");
                break;
            }
            Ok(_) => {
                let trimmed_line = line.trim(); // Remove surrounding whitespace, including '\n'
                if trimmed_line.is_empty() {
                    // Empty line signifies end of header
                    println!("--- End of Response Header ---");
                    break;
                }
                println!("Received: {}", trimmed_line);
                response_header_lines.push(trimmed_line.to_string());
                lines_read += 1;
                // Optional: Add a safety break if too many lines are received
                if lines_read > 10 {
                     eprintln!("Warning: Received too many header lines, stopping read.");
                     break;
                }
            }
            Err(e) => {
                // Handle specific errors like timeout if needed
                 if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut {
                    eprintln!("Error: Timeout waiting for server response.");
                 } else {
                    eprintln!("Error reading response line: {}", e);
                 }
                return Err(e); // Propagate other I/O errors
            }
        }
    }

    // Optional: Basic validation of the response
    if response_header_lines.is_empty() {
        eprintln!("Warning: No valid response header received.");
        // Consider returning an error if a valid response is mandatory
        // return Err(io::Error::new(io::ErrorKind::InvalidData, "No response header received"));
    } else if response_header_lines[0] != RESPONSE_VERSION {
        eprintln!(
            "Warning: Received unexpected response version '{}', expected '{}'",
            response_header_lines[0], RESPONSE_VERSION
        );
         // Consider returning an error
         // return Err(io::Error::new(io::ErrorKind::InvalidData, "Unexpected response version"));
    }

    // Further parsing of Status, Message etc. could go here if needed

    Ok(())
}

use std::{
    env, // For command-line arguments
    fs::File,
    io::{self, BufRead, BufReader, Read, Write}, // Input/output operations
    net::TcpStream, // TCP stream for network communication
    path::Path,     // Path manipulation
    process,        // For exiting the process
    time::Duration, // For setting timeouts
    cmp::min, // Used for min in manual chunking loop (if needed)
};

// --- Configuration ---
const SERVER_ADDR: &str = "127.0.0.1:6969"; // Server address and port (matches server)
const PROTOCOL_VERSION: &str = "PROTO/1.0"; // Client protocol version
const RESPONSE_VERSION: &str = "RESPONSE/1.0"; // Expected response protocol version
const BUFFER_SIZE: usize = 4096; // Size of chunks for sending/receiving file data
const TIMEOUT_SECS: u64 = 10; // Connection and read/write timeout in seconds

#[derive(Debug, PartialEq)]
enum ClientCommand {
    Upload { filepath: String },
    Download { filename: String },
}

fn main() -> io::Result<()> {
    // --- 1. Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    let command = parse_args(&args)?;

    // --- 2. Establish TCP Connection ---
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


    // --- 3. Execute Command (Upload or Download) ---
    match command {
        ClientCommand::Upload { filepath } => {
            println!("Executing UPLOAD command for file: {}", filepath);
            if let Err(e) = execute_upload(&mut stream, &filepath) {
                eprintln!("Error during upload: {}", e);
                // Attempt to close stream gracefully even on error
                let _ = stream.shutdown(std::net::Shutdown::Both);
                process::exit(1);
            }
        }
        ClientCommand::Download { filename } => {
            println!("Executing DOWNLOAD command for file: {}", filename);
             if let Err(e) = execute_download(&mut stream, &filename) {
                eprintln!("Error during download: {}", e);
                // Attempt to close stream gracefully even on error
                let _ = stream.shutdown(std::net::Shutdown::Both);
                process::exit(1);
            }
        }
    }


    // --- 4. Close Connection (implicitly done when stream goes out of scope) ---
    println!("Closing connection.");
    // Optionally, explicitly shut down write side:
    // stream.shutdown(std::net::Shutdown::Write)?;

    Ok(()) // Indicate successful execution
}

/// Parses command line arguments to determine the client command.
fn parse_args(args: &[String]) -> io::Result<ClientCommand> {
    if args.len() < 3 {
        eprintln!("Usage: {} <command> <argument>", args[0]);
        eprintln!("Commands:");
        eprintln!("  upload <filepath>  - Upload a file to the server.");
        eprintln!("  download <filename> - Download a file from the server.");
        process::exit(1);
    }

    let command_str = &args[1];
    let argument = &args[2];

    match command_str.to_lowercase().as_str() {
        "upload" => Ok(ClientCommand::Upload { filepath: argument.clone() }),
        "download" => Ok(ClientCommand::Download { filename: argument.clone() }),
        _ => {
            eprintln!("Error: Unknown command '{}'", command_str);
            eprintln!("Usage: {} <command> <argument>", args[0]);
            eprintln!("Commands:");
            eprintln!("  upload <filepath>  - Upload a file to the server.");
            eprintln!("  download <filename> - Download a file from the server.");
            process::exit(1);
        }
    }
}


/// Executes the upload command.
fn execute_upload(stream: &mut TcpStream, filepath_str: &str) -> io::Result<()> {
    let filepath = Path::new(filepath_str);

    // Validate File and Get Metadata
    if !filepath.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("File not found or is not a regular file at '{}'", filepath_str)));
    }

    let filename = match filepath.file_name() {
        Some(name) => name.to_str().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid filename (non-UTF8)."))?,
        None => return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("Could not extract filename from path '{}'", filepath_str))),
    };

    let file_metadata = filepath.metadata()?;
    let filesize = file_metadata.len(); // Get file size in bytes

    println!("Preparing to upload '{}' ({} bytes)", filename, filesize);

    // Construct the Header
    let header = format!(
        "{}\nCOMMAND: UPLOAD\nFILENAME: {}\nFILESIZE: {}\n\n", // Double \n signifies end
        PROTOCOL_VERSION, filename, filesize
    );

    // Send Header and File Data
    send_header(stream, &header)?;
    send_file_data(stream, filepath, filesize)?;

    // Receive and Print Server Response
    println!("Waiting for server response...");
    receive_response(stream)?;

    Ok(())
}

/// Executes the download command.
fn execute_download(stream: &mut TcpStream, filename: &str) -> io::Result<()> {
    // Construct the Header for download request
    // Filesize is NOT included in the download request header
    let header = format!(
        "{}\nCOMMAND: DOWNLOAD\nFILENAME: {}\n\n", // Double \n signifies end
        PROTOCOL_VERSION, filename
    );

    // Send Download Request Header
    send_header(stream, &header)?;

    // Receive and Parse Server Response Header
    println!("Waiting for server response header for download...");
    let mut reader = BufReader::new(stream);
    let mut response_header_lines: Vec<String> = Vec::new();

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                 return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Connection closed by server before receiving full response header."));
            }
            Ok(_) => {
                let trimmed_line = line.trim();
                if trimmed_line.is_empty() {
                    println!("--- End of Response Header ---");
                    break;
                }
                println!("Received header line: {}", trimmed_line);
                response_header_lines.push(trimmed_line.to_string());
            }
            Err(e) => {
                eprintln!("Error reading response header line: {}", e);
                return Err(e);
            }
        }
    }

    // Get the stream back from the reader to read file data
    let mut stream_for_file = reader.into_inner();

    // Parse Response Header
    if response_header_lines.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No response header received from server."));
    }

    if response_header_lines[0] != RESPONSE_VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Received unexpected response version '{}', expected '{}'", response_header_lines[0], RESPONSE_VERSION)));
    }

    let mut status: Option<String> = None;
    let mut response_filename: Option<String> = None;
    let mut response_filesize: Option<u64> = None; // Filesize from server response is u64

    for line in response_header_lines.iter().skip(1) { // Skip version line
        if let Some(s) = line.strip_prefix("Status: ") {
            status = Some(s.to_string());
        } else if let Some(f) = line.strip_prefix("Filename: ") {
            response_filename = Some(f.to_string());
        } else if let Some(s) = line.strip_prefix("Filesize: ") {
            match s.parse::<u64>() {
                Ok(size) => response_filesize = Some(size),
                Err(_) => return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Invalid Filesize format in response header: {}", s))),
            }
        }
    }

    // Process Status
    match status.as_deref() {
        Some("OK") => {
            println!("Server Status: OK");
            let downloaded_filename = response_filename.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Filename missing in OK response header."))?;
            let downloaded_filesize = response_filesize.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Filesize missing in OK response header."))?;

            println!("Attempting to download file '{}' ({} bytes)", downloaded_filename, downloaded_filesize);

            // Receive and Save File Data
            receive_and_save_downloaded_file(&mut stream_for_file, &downloaded_filename, downloaded_filesize)?;
            println!("File '{}' downloaded successfully.", downloaded_filename);
        }
        Some("ERROR") => {
            let error_message = response_header_lines.iter()
                .find_map(|line| line.strip_prefix("Message: "))
                .unwrap_or("No error message provided by server.");
            eprintln!("Server Status: ERROR");
            eprintln!("Server Message: {}", error_message);
            return Err(io::Error::new(io::ErrorKind::Other, format!("Server reported error: {}", error_message)));
        }
        _ => {
             return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Received unexpected status from server: {:?}", status)));
        }
    }


    Ok(())
}


/// Sends the header bytes to the stream.
fn send_header(stream: &mut TcpStream, header: &str) -> io::Result<()> {
    println!("Sending header:\n---START---\n{}---END---", header.trim()); // Trim for cleaner log
    stream.write_all(header.as_bytes())?;
    stream.flush()?; // Ensure header is sent
    println!("Header sent.");
    Ok(())
}

/// Sends the file content from the given filepath to the stream in chunks.
fn send_file_data(stream: &mut TcpStream, filepath: &Path, filesize: u64) -> io::Result<()> {
    println!("Sending file data ({} bytes)...", filesize);
    let mut file = File::open(filepath)?;
    let mut buffer = vec![0; BUFFER_SIZE]; // Use BUFFER_SIZE for chunking
    let mut bytes_sent: u64 = 0;

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // End of file
        }
        stream.write_all(&buffer[..bytes_read])?;
        bytes_sent += bytes_read as u64;
        // Optional: print progress
        // println!("Sent {}/{} bytes...", bytes_sent, filesize);
    }

    // Ensure all buffered data is sent
    stream.flush()?;

    println!("File data sent ({} bytes).", bytes_sent);

    if bytes_sent != filesize {
         eprintln!(
            "Warning: Sent {} bytes, but expected file size was {}. Connection may have closed prematurely.",
             bytes_sent, filesize
         );
         // Depending on requirements, you might return an error here.
    }

    Ok(())
}

/// Receives exactly `filesize` bytes from the stream and saves them to a local file.
fn receive_and_save_downloaded_file(stream: &mut TcpStream, filename: &str, filesize: u64) -> io::Result<()> {
    // Prevent path traversal when saving the downloaded file locally
    let local_path = Path::new(filename);
     if local_path.components().any(|comp| comp == std::path::Component::ParentDir) || local_path.is_absolute() {
         eprintln!("Security Alert: Attempted path traversal when saving file: {}", filename);
         return Err(io::Error::new(
             io::ErrorKind::InvalidInput,
             "Invalid filename specified by server (potential path traversal)"
         ));
    }


    println!("Attempting to create local file for download: {:?}", local_path);
    let mut file = match File::create(&local_path) {
        Ok(f) => {
            println!("Successfully created local file: {:?}", local_path);
            f
        },
        Err(e) => {
            eprintln!("Error creating local file {:?}: {}", local_path, e);
            return Err(e);
        }
    };

    // Read exactly `filesize` bytes from the stream
    let mut received_bytes = 0;
    let mut buffer = vec![0; BUFFER_SIZE]; // Use BUFFER_SIZE for chunking

    println!("Starting to receive file data (expecting {} bytes)...", filesize);

    // Use io::copy for potentially simpler and more efficient transfer
    // Note: io::copy returns u64, which matches filesize type
    let bytes_copied = io::copy(stream, &mut file)?;
    received_bytes = bytes_copied;


    // Alternative manual loop (less concise than io::copy):
    /*
    while received_bytes < filesize {
        let bytes_to_read = min(buffer.len() as u64, filesize - received_bytes) as usize; // Ensure we don't read more than needed
        if bytes_to_read == 0 { // Should not happen if filesize is > received_bytes
             break;
        }

        let bytes_read = stream.read(&mut buffer[..bytes_to_read])?;

        if bytes_read == 0 {
             eprintln!("Connection closed prematurely. Expected {} bytes, received {}.", filesize, received_bytes);
             return Err(io::Error::new(
                 io::ErrorKind::UnexpectedEof,
                 format!("Connection closed before receiving expected {} bytes. Received only {}.", filesize, received_bytes)
             ));
        }

        file.write_all(&buffer[..bytes_read])?;
        received_bytes += bytes_read as u64;
        // println!("Received {}/{} bytes...", received_bytes, filesize);
    }
    */


    // Ensure all buffered data is written to the file
    file.flush()?;
    println!("Finished receiving and saving file data. Total bytes received: {}", received_bytes);

    if received_bytes != filesize {
         eprintln!(
            "Warning: Received {} bytes for file '{}', but expected file size was {}. Connection may have closed prematurely.",
             received_bytes, filename, filesize
         );
         // Depending on requirements, you might return an error here.
    }


    Ok(())
}

/// Receives and processes the server's response header (for upload).
/// Assumes the stream is positioned at the start of the response header.
fn receive_response(stream: &mut TcpStream) -> io::Result<()> {
    // Use BufReader for efficient line-based reading
    let mut reader = BufReader::new(stream);
    let mut response_header_lines: Vec<String> = Vec::new();

    loop {
        let mut line = String::new();
        // Read one line, including the newline character
        match reader.read_line(&mut line) {
            Ok(0) => {
                // Connection closed prematurely
                eprintln!("Warning: Connection closed by server before receiving full response header.");
                break; // Exit loop, handle empty lines later
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
            }
            Err(e) => {
                eprintln!("Error reading response line: {}", e);
                return Err(e); // Propagate other I/O errors
            }
        }
    }

    // Basic validation of the response header structure
    if response_header_lines.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "No response header received from server."));
    } else if response_header_lines[0] != RESPONSE_VERSION {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Received unexpected response version '{}', expected '{}'", response_header_lines[0], RESPONSE_VERSION)));
    }

    // Parse Status and optional Message
    let mut status: Option<String> = None;
    let mut message: Option<String> = None;

    for line in response_header_lines.iter().skip(1) { // Skip version line
        if let Some(s) = line.strip_prefix("Status: ") {
            status = Some(s.to_string());
        } else if let Some(m) = line.strip_prefix("Message: ") {
            message = Some(m.to_string());
        }
    }

    // Act based on Status
    match status.as_deref() {
        Some("OK") => {
            println!("\nServer Status: OK. File uploaded successfully!");
            if let Some(msg) = message {
                println!("Server message: {}", msg);
            }
        }
        Some("ERROR") => {
            eprintln!("\nServer Status: ERROR. File upload failed!");
            if let Some(msg) = message {
                eprintln!("Server message: {}", msg);
            } else {
                 eprintln!("No error message provided by the server.");
            }
            return Err(io::Error::new(io::ErrorKind::Other, "Server reported an error during upload"));
        }
        _ => {
            eprintln!("\nReceived unexpected status from server: {:?}", status);
            if let Some(msg) = message {
                eprintln!("Server message: {}", msg);
            }
            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Unexpected server status: {:?}", status)));
        }
    }

    Ok(())
}

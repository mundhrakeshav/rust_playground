use rustbook_webserver::ThreadPool;
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};
fn main() {
    let tcp_listener: TcpListener =
        TcpListener::bind("127.0.0.1:7878").expect("unable to bind tcp listener");
    println!("tcp_listener");
    let pool = ThreadPool::new(5);
    for stream in tcp_listener.incoming() {
        let mut stream: TcpStream = stream.unwrap();

        pool.exec(move || {
            handle_connection(&mut stream);
            println!("Connection established!");
        })
        // thread::spawn(move || {
        //     handle_connection(&mut stream);
        //     println!("Connection established!");
        // });
    }
}

fn handle_connection(stream: &mut TcpStream) {
    let buf_reader: BufReader<&mut TcpStream> = BufReader::new(stream);
    let request_line: String = buf_reader.lines().next().unwrap().unwrap();
    let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
        thread::sleep(Duration::from_secs(5));
        ("HTTP/1.1 200 OK", "hwlo.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "404.html")
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

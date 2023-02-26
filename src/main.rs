use std::{
    env,
    io::{BufRead, BufReader, Write},
    net, time::Duration,
};
use multithreaded_web_server::ThreadPool;

fn main() {
    let mut args = env::args();

    // Skip default arg
    args.next();

    let server_addr = args.next().unwrap();
    let server = net::TcpListener::bind(server_addr).unwrap_or_else(|err| {
        eprintln!("Error: {err}");
        std::process::exit(1);
    });

    let threads_num = match args.next() {
        Some(v) => match v.parse::<usize>() {
            Ok(num) => num,
            Err(err) => {
                eprintln!("Failed to ` number of threads. Error: {err}");
                std::process::exit(1);
            }
        }
        None => std::thread::available_parallelism().unwrap().into()
    };  

    let mut pool = ThreadPool::new(threads_num);

    for stream in server.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| handle_connection(stream));
    }
}

fn handle_connection(mut socket: net::TcpStream) {
    let buf_reader = BufReader::new(&mut socket);
    let status_line_request = buf_reader.lines().next().unwrap().unwrap();
    
    let (status_line, content) = match &status_line_request[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            std::thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 Not Found", "404.html")
    };

    let content = std::fs::read_to_string(content).unwrap();
    let content_length = content.len();
    let response = std::format!("{status_line}\r\nContent-Length: {content_length}\r\n\r\n{content}").to_string();
    socket.write_all(response.as_bytes()).unwrap();
}

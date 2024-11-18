use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream}, thread, time::Duration,
};

use hello::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);

    // take 2 requests then shutdown
    // it will shudown because end of main() is reached, meaning the pool goes out of scope and the `drop` implementation will run
    for stream in listener.incoming().take(2) {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }

    println!("Shutting down server...");
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    // HTTP Responses have this format:
    // 
    // HTTP-Version Status-Code Reason-Phrase CRLF
    // headers CRLF
    // message-body

    // CRLF stands for carriage return and line feed, which are terms from the typewriter days!) The CRLF sequence can also be written as \r\n, where \r is a carriage return and \n is a line feed. 
    
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();

    let response =
        format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();

}
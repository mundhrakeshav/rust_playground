use clap::Parser;
use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};
use trust_dns_client::{
    op::{Message, MessageType, OpCode, Query},
    rr::{Name, RecordType},
    serialize::binary::{BinEncodable, BinEncoder},
};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short = 's', long, default_value_t = ("1.1.1.1").to_string())]
    dns_server: String,
    #[arg(short = 'd', long)]
    domain_name: String,
}

fn main() {
    let args = Args::parse();

    let domain_name = Name::from_ascii(&args.domain_name).expect("Invalid domain name");
    let dns_server: SocketAddr = format!("{}:53", args.dns_server)
        .parse()
        .expect("Invalid DNS server address");

    let mut request_as_bytes: Vec<u8> = Vec::with_capacity(512);
    let mut response_as_bytes = vec![0; 512];

    let mut msg = Message::new();
    let msg_id = 100;
    msg.set_id(msg_id)
        .set_message_type(MessageType::Query)
        .add_query(Query::query(domain_name, RecordType::A))
        .set_op_code(OpCode::Query)
        .set_recursion_desired(true);

    let mut encoder = BinEncoder::new(&mut request_as_bytes);
    msg.emit(&mut encoder).expect("failed to encode");

    let localhost = UdpSocket::bind("0.0.0.0:0").expect("Cannot bind to local socket");
    localhost
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    localhost.set_nonblocking(false).unwrap();

    localhost
        .send_to(&request_as_bytes, dns_server)
        .expect("Failed to send DNS request");

    let (_amt, _remote) = localhost
        .recv_from(&mut response_as_bytes)
        .expect("Timeout reached");

    let dns_message = Message::from_vec(&response_as_bytes).expect("Unable to parse response");

    for answer in dns_message.answers() {
        if let Some(ip) = answer.data().and_then(|r| r.as_a()) {
            println!("ip: {}", ip);
        }
    }
}

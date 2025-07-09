use ssh2::Session;
use std::net::TcpStream;

pub fn create_session(user: &str, host: &str) -> Session {
    let tcp = TcpStream::connect(format!("{host}:22")).unwrap();
    let mut session = Session::new().unwrap();
    session.set_tcp_stream(tcp);
    session.handshake().unwrap();
    session.userauth_agent(user).unwrap();
    session
}

#[macro_use]
extern crate serde_derive;

#[cfg(test)]
mod stratum {
    extern crate serde;
    extern crate serde_json;

    use serde::de::DeserializeOwned;

    use std::io::{self, Read, Write, BufReader, BufRead};
    use std::net::TcpStream;

    struct Stratum {
        addr: String,
        pub stream: Option<TcpStream>,
    }

    struct User<'a> (
        pub &'a str,
        pub &'a str,
    );

    impl<'a> User<'a> {
        pub fn to_vec(&self) -> Vec<String> {
            vec![String::from(self.0), String::from(self.1)]
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    enum _Result {
        R(bool),
        T(Vec<String>),
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    enum Message {
        C {
            id: i32,
            method: String,
            params: Vec<String>,
        },

        S {
            id: i32,
            result: _Result,
            error: Vec<String>,
        },
    }

    impl Stratum {
        pub fn new(addr: &str) -> Self {
            Self {
                addr: String::from(addr),
                stream: None,
            }
        }

        pub fn try_connect(&mut self) {
            self.stream = match TcpStream::connect(&self.addr) {
                Ok(s) => Some(s),
                Err(e) => {
                    println!("connect to {} failed: {}", &self.addr, e);
                    None
                }
            }
        }

        pub fn try_send(&mut self, msg: &Message) {
            match self.stream {
                Some(ref mut s) => {
                    s.write(&serde_json::to_vec(&msg).unwrap());
                    s.write(&['\n' as u8]);
                }
                None => println!("no connect!")
            };
        }

        pub fn try_read(&mut self) -> Result<Message, serde_json::Error> {
            match self.stream {
                Some(ref s) => {
                    let mut buf = String::new();
                    let mut bufr = BufReader::new(s);
                    bufr.read_line(&mut buf);
                    serde_json::from_str(&buf)
                }
                None => panic!()
            }
        }

        pub fn subscribe(&mut self) {
            let msg = Message::C {
                id: 1,
                method: String::from("mining.subscribe"),
                params: vec![],
            };

            self.try_send(&msg);
        }
    }

    #[test]
    fn connect_to_tcp() {
        let mut s = Stratum::new("127.0.0.1:7878");
        s.try_connect();
        s.subscribe();
        println!("{:?}", s.try_read());
    }

    #[test]
    fn serialize_json_data() {
        fn to_json_str(msg: &Message) -> String {
            serde_json::to_string(&msg).unwrap()
        }

        let msg = Message::C {
            id: 1,
            method: String::from("mining.subscribe"),
            params: vec![],
        };
        assert_eq!(r#"{"id":1,"method":"mining.subscribe","params":[]}"#, to_json_str(&msg));

        let msg = Message::S {
            id: 2,
            result: _Result::R(true),
            error: vec![],
        };
        assert_eq!(r#"{"id":2,"result":true,"error":[]}"#, to_json_str(&msg));

        let user = User("user1", "password");

        let msg = Message::C {
            id: 3,
            method: String::from("mining.authorize"),
            params: user.to_vec(),
        };
        assert_eq!(r#"{"id":3,"method":"mining.authorize","params":["user1","password"]}"#,
                   to_json_str(&msg));
    }
}

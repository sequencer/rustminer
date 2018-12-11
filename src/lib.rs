#[cfg(test)]
#[macro_use]
extern crate serde_derive;

#[cfg(test)]
#[macro_use]
extern crate serde_json;

#[cfg(test)]
mod stratum {
    use std::io::prelude::*;
    use std::io::{self, BufReader, BufRead};
    use std::net::TcpStream;

    use self::msg::JsonToString;

    pub struct Pool {
        addr: String,
        stream: Option<TcpStream>,
        msgid: u32
    }

    mod msg {
        #[derive(Serialize, Debug)]
        pub struct Client<'a> {
            pub id: u32,
            pub method: String,
            pub params: Vec<&'a str>,
        }

        #[derive(Serialize, Deserialize, Debug)]
        pub struct Server {
            pub id: u32,
            pub result: serde_json::Value,
            pub error: serde_json::Value,
        }

        pub trait JsonToString: serde::Serialize {
            fn to_string(&self) -> serde_json::Result<String> {
                serde_json::to_string(&self)
            }
        }

        impl<T: serde::Serialize> JsonToString for T {}
    }

    impl Pool {
        pub fn new(addr: &str) -> Self {
            Self {
                addr: String::from(addr),
                stream: None,
                msgid: 0
            }
        }

        fn msgid(&mut self) -> u32 {
            self.msgid = self.msgid + 1;
            self.msgid
        }

        pub fn try_connect(&mut self) -> io::Result<&TcpStream> {
            match self.stream {
                Some(ref s) => Ok(s),
                None => {
                    self.stream = Some(TcpStream::connect(&self.addr)?);
                    Ok(self.stream.as_ref().unwrap())
                }
            }
        }

        pub fn try_send<T: serde::Serialize>(&mut self, msg: T) -> io::Result<usize> {
            let mut data = serde_json::to_vec(&msg).unwrap();
            data.push(b'\n');
            self.try_connect()?.write(&data)
        }

        pub fn try_read(&mut self) -> io::Result<msg::Server> {
            let mut buf = String::new();
            let mut bufr = BufReader::new(self.try_connect()?);
            bufr.read_line(&mut buf).unwrap();
            println!("{}", &buf);
            Ok(serde_json::from_str(&buf)?)
        }

        pub fn subscribe(&mut self) -> io::Result<usize> {
            let msg = msg::Client {
                id: self.msgid(),
                method: String::from("mining.subscribe"),
                params: vec![],
            };
            self.try_send(&msg)
        }

        pub fn authorize(&mut self, user: &str, pass: &str) -> io::Result<usize> {
            let msg = msg::Client {
                id: self.msgid(),
                method: String::from("mining.authorize"),
                params: vec![user, pass]
            };
            self.try_send(&msg)
        }
    }

    #[test]
    fn connect_to_tcp() {
        let mut s = Pool::new("cn.ss.btc.com:1800");
        let ret = s.try_connect();
        println!("1,{:?}", ret);
        let ret = s.subscribe();
        println!("2,{:?}", ret);
        let ret = s.try_read();
        println!("3,{:?}", ret);
        let ret = s.authorize("username","");
        println!("4,{:?}", ret);
        let ret = s.try_read();
        println!("5,{:?}", ret);
    }

    #[test]
    fn serialize_json_data() {
        let msg = msg::Client {
            id: 1,
            method: String::from("mining.subscribe"),
            params: vec![],
        };
        assert_eq!(r#"{"id":1,"method":"mining.subscribe","params":[]}"#, &msg.to_string().unwrap());

        let msg = msg::Server {
            id: 2,
            result: json!(true),
            error: json!(null),
        };
        assert_eq!(r#"{"id":2,"result":true,"error":null}"#, &msg.to_string().unwrap());

        let msg = msg::Client {
            id: 3,
            method: String::from("mining.authorize"),
            params: vec!["user1", "password"],
        };
        assert_eq!(r#"{"id":3,"method":"mining.authorize","params":["user1","password"]}"#,
                   &msg.to_string().unwrap());
    }
}

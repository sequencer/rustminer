use super::*;

#[test]
fn connect_to_tcp() {
    let mut pool = Pool::new("cn.ss.btc.com:1800");
    let ret = pool.try_connect();
    println!("1,{:?}", ret);
    let ret = pool.subscribe();
    println!("2,{:?}", ret);
    let ret = pool.try_read();
    println!("3,{}", ret);
    let ret = pool.authorize("h723n8m.001", "");
    println!("4,{:?}", ret);
    for received in pool.receiver() {
        println!("received: {}", received);
    }
    pool.join_all();
}

#[test]
fn serialize_json_data() {
    use self::ToString;

    let msg = Action {
        id: Some(1),
        method: String::from("mining.subscribe"),
        params: Params::None(vec![]),
    };
    assert_eq!(r#"{"id":1,"method":"mining.subscribe","params":[]}"#, &msg.to_string().unwrap());

    let msg = Action {
        id: Some(3),
        method: String::from("mining.authorize"),
        params: Params::User([String::from("user1"), String::from("password")]),
    };
    assert_eq!(r#"{"id":3,"method":"mining.authorize","params":["user1","password"]}"#,
               &msg.to_string().unwrap());
}

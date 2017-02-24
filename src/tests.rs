// Copyright 2017 Dario Domizioli ("hhexo").
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(test)]
mod tests {

use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use std::collections::HashMap;

use Actor;
use MessageType;
use MessageDatum;
use Message;
use ActorAddress;
use spawn;

pub struct Echo;

const TEST : &'static str = ":test";

impl Actor for Echo {
    fn process_message(&mut self, message: Message, myself: &ActorAddress) {
        match *message.get_type() {
            MessageType::Init => {
                Message::init().with_sender(myself)
                               .with_datum(message.get_datum().clone())
                               .send_to(message.get_sender());
            },
            MessageType::Stop => {
                Message::stop().with_sender(myself)
                               .with_datum(message.get_datum().clone())
                               .send_to(message.get_sender());
            },
            MessageType::Custom(s) => {
                Message::custom(s).with_sender(myself)
                                  .with_datum(message.get_datum().clone())
                                  .send_to(message.get_sender());
            },
        }
    }
}

#[test]
fn basic_check() {
    let (tx, rx) = mpsc::channel();
    let fake_actor = ActorAddress { endpoint: tx };
    let worker = spawn(Echo);

     // Hopefully this is enough for the init message to get to the worker
    thread::sleep(Duration::from_millis(500));

    Message::custom(TEST).with_sender(&fake_actor).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_i64(-123).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_u64(456).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_f64(123.456).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_str("blah").send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_map(HashMap::new()).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_act(&worker).send_to(&worker);
    Message::stop().with_sender(&fake_actor).send_to(&worker);

     // Hopefully this is enough for the stop message to get to the worker
    thread::sleep(Duration::from_millis(500));

    let mut m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::Void => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::I64(-123) => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::U64(456) => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::F64(123.456) => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::Str(ref s) => { assert_eq!(s, "blah"); }, // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::Map(ref m) => { assert!(m.is_empty()) }, // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Custom(TEST));
    match *m.get_datum() {
        MessageDatum::Act(_) => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }
}

}


// The textbook tests are based on the book called "Seven Concurrency Models
// In Seven Weeks". Chapter 5 of the book is about the actor model. I am trying
// to replicate the examples within with the mecha library.

#[cfg(test)]
mod textbook_tests {

use std::thread;
use std::time::Duration;
use std::collections::HashMap;

use Actor;
use MessageType;
use MessageDatum;
use Message;
use ActorAddress;
use spawn;

struct Talker;

const GREET: &'static str = ":greet";
const PRAISE: &'static str = ":praise";
const CELEBRATE: &'static str = ":celebrate";

impl Actor for Talker {
    fn process_message(&mut self, message: Message, _: &ActorAddress) {
        match *message.get_type() {
            MessageType::Custom(GREET) => {
                match *message.get_datum() {
                    MessageDatum::Str(ref s) => {
                        println!("Hello {}!", s);
                    },
                    _ => (),
                }
            },
            MessageType::Custom(PRAISE) => {
                match *message.get_datum() {
                    MessageDatum::Str(ref s) => {
                        println!("{}, you're amazing!", s);
                    },
                    _ => (),
                }
            },
            MessageType::Custom(CELEBRATE) => {
                match *message.get_datum() {
                    MessageDatum::Map(ref m) => {
                        let name = m.get("name").unwrap().clone().as_str().unwrap();
                        let age = m.get("age").unwrap().clone().as_u64().unwrap();
                        println!("Here's to another {} years, {}!", age, name);
                    },
                    _ => (),
                }
            },
            _ => (),
        }
    }
}

#[test]
fn test_talker() {
    let worker = spawn(Talker);

    Message::custom(GREET).with_str("Hewey").send_to(&worker);
    Message::custom(PRAISE).with_str("Dewey").send_to(&worker);
    let mut m = HashMap::new();
    m.insert("name".to_string(), MessageDatum::from("Louie"));
    m.insert("age".to_string(), MessageDatum::from(16u64));
    Message::custom(CELEBRATE).with_map(m).send_to(&worker);

    Message::stop().send_to(&worker);

     // Hopefully this is enough for the stop message to get to the worker
    thread::sleep(Duration::from_millis(500));
}

}

// Copyright 2017 Dario Domizioli ("hhexo").
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// These tests are based on the book called "Seven Concurrency Models
// In Seven Weeks". Chapter 5 of the book is about the actor model. I am trying
// to replicate the exercises in the book by writing equivalent tests for the
// mecha library.

#[cfg(test)]
mod chapter5_day1 {

use std::sync::mpsc;
use std::collections::HashMap;

use Actor;
use MessageType;
use MessageDatum;
use Message;
use ActorAddress;
use spawn_link;

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

struct Issuer;

impl Actor for Issuer {
    fn process_message(&mut self, message: Message, myself: &ActorAddress) {
        match *message.get_type() {
            MessageType::Init => {
                let worker = spawn_link(Talker, myself);
                Message::custom(GREET).with_str("Hewey").send_to(&worker);
                Message::custom(PRAISE).with_str("Dewey").send_to(&worker);
                let mut m = HashMap::new();
                m.insert("name".to_string(), MessageDatum::from("Louie"));
                m.insert("age".to_string(), MessageDatum::from(16u64));
                Message::custom(CELEBRATE).with_map(m).send_to(&worker);
                Message::shutdown().send_to(&worker);
            },
            MessageType::Exited => {
                // This means our worker has exited, we can exit too. Send
                // ourselves a Shutdown.
                Message::shutdown().send_to(myself);
            },
            _ => ()
        }
    }
}

#[test]
fn test_talker() {
    let (tx, rx) = mpsc::channel();
    let fake_actor = ActorAddress::new(tx);
    spawn_link(Issuer, &fake_actor);

    // Now let's receive the Exited message.
    let m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Exited);
    match *m.get_datum() {
        MessageDatum::Void => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }
}

}

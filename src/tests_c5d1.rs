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

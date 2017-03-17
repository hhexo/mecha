// Copyright 2017 Dario Domizioli ("hhexo").
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.


#[cfg(test)]
mod chapter5_day1 {
//! These tests are based on the book called "Seven Concurrency Models
//! In Seven Weeks". Chapter 5 of the book is about the actor model. I am trying
//! to replicate the exercises in the book by writing equivalent tests for the
//! mecha library.


use MessageType;
use MessageDatum;
use Message;
use ActorAddress;
use Actor;
use Stateless;

use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::sync::mpsc;


const GREET: &'static str = ":greet";
const PRAISE: &'static str = ":praise";
const CELEBRATE: &'static str = ":celebrate";

#[test]
fn test_talker() {
    let (tx, rx) = mpsc::channel();
    let initiator = ActorAddress::new(tx.clone());

    let worker = Actor::new().with_state(Stateless)
        .with_match(|msg, _| {
            match *msg.get_type() {
                MessageType::Custom(GREET) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("Hello {}",
                msg.get_datum().clone().as_str().unwrap());
            Ok(())
        })
        .with_match(|msg, _| {
            match *msg.get_type() {
                MessageType::Custom(PRAISE) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("{}, you're amazing",
                msg.get_datum().clone().as_str().unwrap());
            Ok(())
        })
        .with_match(|msg, _| {
            match *msg.get_type() {
                MessageType::Custom(CELEBRATE) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("Here's to another {} years, {}", 
                msg.get_datum().clone().as_map().unwrap().get("age").unwrap().clone().as_i64().unwrap(),
                msg.get_datum().clone().as_map().unwrap().get("name").unwrap().clone().as_str().unwrap());
            Ok(())
        })
        .spawn_link(&initiator);

    Message::custom(GREET).with_sender(&initiator)
                          .with_str("Huey")
                          .send_to(&worker);
    Message::custom(PRAISE).with_sender(&initiator)
                           .with_str("Dewey")
                           .send_to(&worker);
    let mut map = HashMap::new();
    map.insert("age".to_string(), MessageDatum::from(16i64));
    map.insert("name".to_string(), MessageDatum::from("Louie"));
    Message::custom(CELEBRATE).with_sender(&initiator)
                          .with_map(map)
                          .send_to(&worker);

    thread::sleep(Duration::from_millis(500));
    Message::shutdown().with_sender(&initiator).send_to(&worker);

    let msg = rx.recv().unwrap();
    assert_eq!(*msg.get_type(), MessageType::Exited);
    match *msg.get_datum() {
        MessageDatum::Void => (),
        _ => { assert!(false, "Unexpected message datum"); }
    }
}

#[derive(Default)]
struct CounterState { count: i64 }
struct CounterApi { counter: ActorAddress }

const COUNT: &'static str = ":count";
const COUNT_ACK: &'static str = ":count_ack";

impl CounterApi {
    pub fn new(parent: &ActorAddress) -> CounterApi {
        let act = Actor::new().with_state(CounterState {count: 0})
        .with_match(|msg, _| {
            match *msg.get_type() {
                MessageType::Custom(COUNT) => true,
                _ => false
            }
        })
        .with_action(|msg, state, myself| {
            state.count += 1;
            Message::custom(COUNT_ACK).with_sender(myself)
                                      .with_i64(state.count)
                                      .send_to(msg.get_sender());
            Ok(())
        })
        .spawn_link(parent);

        CounterApi {
            counter: act
        }
    }

    pub fn next(&self) -> i64 {    
        let (tx, rx) = mpsc::channel();
        let initiator = ActorAddress::new(tx);
        Message::custom(COUNT).with_sender(&initiator)
                              .send_to(&self.counter);
        rx.recv().unwrap().get_datum().clone().as_i64().unwrap()
    }

    pub fn shutdown(&self) {
        Message::shutdown().send_to(&self.counter);
    }
}

#[test]
fn test_counter() {
    let (tx, rx) = mpsc::channel();
    let initiator = ActorAddress::new(tx.clone());

    let capi = CounterApi::new(&initiator);

    assert_eq!(capi.next(), 1i64);
    assert_eq!(capi.next(), 2i64);
    assert_eq!(capi.next(), 3i64);

    capi.shutdown();

    let msg = rx.recv().unwrap();
    assert_eq!(*msg.get_type(), MessageType::Exited);
    match *msg.get_datum() {
        MessageDatum::Void => (),
        _ => { assert!(false, "Unexpected message datum"); }
    }
}




}

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
            MessageType::Exited => {
                Message::exited().with_sender(myself)
                                 .with_datum(message.get_datum().clone())
                                 .send_to(message.get_sender());
            },
            MessageType::Link => {
                Message::link().with_sender(myself)
                               .with_datum(message.get_datum().clone())
                               .send_to(message.get_sender());
            },
            MessageType::Shutdown => {
                Message::shutdown().with_sender(myself)
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

    Message::link().with_sender(&fake_actor).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_i64(-123).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_u64(456).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_f64(123.456).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_str("blah").send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_map(HashMap::new()).send_to(&worker);
    Message::custom(TEST).with_sender(&fake_actor).with_act(&worker).send_to(&worker);
    Message::shutdown().with_sender(&fake_actor).send_to(&worker);

     // Hopefully this is enough for the stop message to get to the worker
    thread::sleep(Duration::from_millis(500));

    // First we're going to receive the echoed Link message.
    let mut m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Link);
    match *m.get_datum() {
        MessageDatum::Void => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }

    // Then the custom messages.
    m = rx.recv().unwrap();
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

    // Finally we are going to receive the echoed Shutdown message and then the
    // Exited message because of the link.
    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Shutdown);
    match *m.get_datum() {
        MessageDatum::Void => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }
    m = rx.recv().unwrap();
    assert_eq!(*m.get_type(), MessageType::Exited);
    match *m.get_datum() {
        MessageDatum::Void => (), // ok
        _ => { assert!(false, "Unexpected message datum"); }
    }
}

}

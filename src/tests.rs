// Copyright 2017 Dario Domizioli ("hhexo").
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(test)]
mod tests {

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

#[test]
fn basic_test() {
    let (tx, rx) = mpsc::channel();
    let initiator = ActorAddress::new(tx.clone());

    let worker = Actor::new().with_state(Stateless)
        .with_match(|msg, _| {
            match *msg.get_type() {
                MessageType::Custom(_) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("{:?}", msg);
            Ok(())
        })
        .spawn_link(&initiator);

    Message::custom("blah").with_sender(&initiator).send_to(&worker);
    Message::custom("blah").with_sender(&initiator).with_i64(-123).send_to(&worker);
    Message::custom("blah").with_sender(&initiator).with_u64(456).send_to(&worker);
    Message::custom("blah").with_sender(&initiator).with_f64(123.456).send_to(&worker);
    Message::custom("blah").with_sender(&initiator).with_str("Hello!").send_to(&worker);
    Message::custom("blah").with_sender(&initiator).with_map(HashMap::new()).send_to(&worker);
    Message::custom("blah").with_sender(&initiator).with_act(&initiator).send_to(&worker);

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
struct CounterState { active: bool, count: i32 }

const INC : &'static str = ":inc";
const ACTIVATE : &'static str = ":activate";

#[test]
fn test_stateful() {
    let (tx, rx) = mpsc::channel();
    let initiator = ActorAddress::new(tx);

    let worker = Actor::new()
        // Initial state
        .with_state(CounterState {
            active: false,
            count: 0
        })
        // Specify increment match and action
        .with_match(|m, state| {
            match *m.get_type() {
                MessageType::Custom(INC) => { state.active },
                _ => false
            }
        })
        .with_action(|_, state, _| {
            state.count += 1;
            println!("The new count is {}", state.count);
            Ok(())
        })
        // Specify activate match and action
        .with_match(|m, _| {
            match *m.get_type() {
                MessageType::Custom(ACTIVATE) => true,
                _ => false
            }
        })
        .with_action(|_, state, _| {
            state.active = true;
            println!("Actor activated!");
            Ok(())
        })
        // Go!
        .spawn_link(&initiator);

    // Let's increment it three times.
    Message::custom(INC).send_to(&worker);
    Message::custom(INC).send_to(&worker);
    Message::custom(INC).send_to(&worker);
    thread::sleep(Duration::from_millis(500));
    // Nothing is really happening so far, we must also activate the actor.
    Message::custom(ACTIVATE).send_to(&worker);
    // Now things should be happening, and they should not be interrupted by
    // this shutdown message because there were still messages in the actor
    // process's mailbox and they are being processed before the shutdown.
    Message::shutdown().send_to(&worker);

    // Now wait for the actor to send the Exited message back to us
    let msg = rx.recv().unwrap();
    assert_eq!(*msg.get_type(), MessageType::Exited);
    match *msg.get_datum() {
        MessageDatum::Void => { println!("Actor exited cleanly."); },
        _ => { println!("Actor must have exited with an error."); }
    }
}




}

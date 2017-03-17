// Copyright 2017 Dario Domizioli ("hhexo").
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Mecha is yet another actor model implementation written in Rust.
//!
//! There are already Rust crates implementing such model. I'll give a shout out
//! to [RobotS](https://github.com/gamazeps/RobotS) and
//! [acto-rs](https://github.com/dbeck/acto-rs), but I suspect there are others,
//! and some building blocks are there in other libraries.
//!
//! The main reason why I am writing this library is as a learning exercise, and
//! it should not be expected to be feature-complete (I wish!) or stable.
//!
//! Do not use this in production code.
//!
//! # Using mecha
//!
//! If you really want to use mecha, feel free to add a git dependency. I
//! haven't gone through the effort of uploading a crate to crates.io yet.
//!
//! `mecha = { git = "https://github.com/hhexo/mecha.git" }`
//!
//! # Examples
//!
//! ## The simple example
//!
//! Here is a quick and dirty example of how to spawn an actor and send
//! messages to it.
//!
//! ```no_run
//! extern crate mecha;
//!
//! fn main() {
//!     let worker = mecha::Actor::new().with_state(mecha::Stateless)
//!         .with_match(|msg, _| {
//!             match *msg.get_type() {
//!                 mecha::MessageType::Custom(_) => true,
//!                 _ => false
//!             }
//!         })
//!         .with_action(|msg, _, _| {
//!             println!("{:?}", msg);
//!             Ok(())
//!         })
//!         .spawn();
//!
//!     // Void message
//!     mecha::Message::custom("blah").send_to(&worker);
//!     // String message
//!     mecha::Message::custom("blah").with_str("Hello!").send_to(&worker);
//!     // Shut down the worker actor
//!     mecha::Message::shutdown().send_to(&worker);
//!
//!     // Wait a bit
//!     use std::thread;
//!     use std::time::Duration;
//!     thread::sleep(Duration::from_millis(500));
//! }
//! ```
//!
//! Generally you will want to wait until the actor process (which is running in
//! another thread) has finished processing the messages and printed the output.
//!
//! For this quick and dirty example we have just waited a bit. However, that is
//! not the correct way to do it. You really want to wait until you know for
//! certain that the actor process has shut down.
//!
//! ## Linking actors
//!
//! Actor processes can be _linked_ to each other. If A is linked to B, then B
//! will send A an `Exited` message whenever it exits due to either a shutdown
//! request (in which case the datum of the message is `Void`) or an error (in
//! which case the datum of the message is a `String` containing an error
//! reason).
//!
//! Your main program is not an actor process, however. How can you link it to
//! the actor process you have spawned?
//!
//! You can create a Rust `mpsc` channel and wrap the sender part of it in an
//! ActorAddress to create a "fake" actor address. Then you can use `spawn_link`
//! instead of `spawn`, so your fake actor will be immediately linked to the
//! spawned worker actor. Once the worker exits, an `Exited` message will be
//! available at the receiving end of the `mpsc` channel, therefore you can wait
//! for it.
//!
//! The following is an example of such pattern:
//!
//! ```
//! extern crate mecha;
//! use std::sync::mpsc;
//!
//! fn main() {
//!     let (tx, rx) = mpsc::channel();
//!     let initiator = mecha::ActorAddress::new(tx);
//!
//!     let worker = mecha::Actor::new().with_state(mecha::Stateless)
//!         .with_match(|msg, _| {
//!             match *msg.get_type() {
//!                 mecha::MessageType::Custom(_) => true,
//!                 _ => false
//!             }
//!         })
//!         .with_action(|msg, _, _| {
//!             println!("{:?}", msg);
//!             Ok(())
//!         })
//!         .spawn_link(&initiator);
//!
//!     // Void message
//!     mecha::Message::custom("blah").send_to(&worker);
//!     // String message
//!     mecha::Message::custom("blah").with_str("Hello!").send_to(&worker);
//!     // Shut down the worker actor
//!     mecha::Message::shutdown().send_to(&worker);
//!
//!     // Now wait for the actor to send the Exited message back to us
//!     let msg = rx.recv().unwrap();
//!     assert_eq!(*msg.get_type(), mecha::MessageType::Exited);
//!     match *msg.get_datum() {
//!         mecha::MessageDatum::Void => { println!("Actor exited cleanly."); },
//!         _ => { println!("Actor must have exited with an error."); }
//!     }
//! }
//! ```
//!
//! It is also possible to link to an actor _after_ it has been spawned. This is
//! done by sending a `Link` message to an existing actor process.
//!
//! ```no_run
//! extern crate mecha;
//! use std::sync::mpsc;
//!
//! fn main() {
//!     let (tx, rx) = mpsc::channel();
//!     let initiator = mecha::ActorAddress::new(tx);
//!
//!     let worker = mecha::Actor::new().with_state(mecha::Stateless)
//!         .with_match(|msg, _| {
//!             match *msg.get_type() {
//!                 mecha::MessageType::Custom(_) => true,
//!                 _ => false
//!             }
//!         })
//!         .with_action(|msg, _, _| {
//!             println!("{:?}", msg);
//!             Ok(())
//!         })
//!         .spawn();
//!
//!     // Link after spawn
//!     mecha::Message::link().with_sender(&initiator).send_to(&worker);
//!
//!     // ...
//! }
//! ```
//!
//! In fact, the `spawn_link` function does nothing more than spawning an actor
//! process and immediately sending a `Link` message to it.
//!
//! ## Stateful actors
//!
//! In the previous examples, the actor process was stateless; however it is
//! possible to have stateful actors so that match clauses can make informed
//! decisions based on state, and actions can modify the actor's state.
//!
//! Here is an example of a counter which is sensitive to two messages: one that
//! increments it only if the counter is "active", and the other one that
//! activates the actor.
//!
//! ```
//! extern crate mecha;
//! use std::sync::mpsc;
//!
//! #[derive(Default)]
//! struct CounterState { active: bool, count: i32 }
//!
//! const INC : &'static str = ":inc";
//! const ACTIVATE : &'static str = ":activate";
//!
//! #[test]
//! fn test_stateful() {
//!     let (tx, rx) = mpsc::channel();
//!     let initiator = mecha::ActorAddress::new(tx);
//!
//!     let worker = mecha::Actor::new()
//!         // Initial state
//!         .with_state(CounterState {
//!             active: false,
//!             count: 0
//!         })
//!         // Specify increment match and action
//!         .with_match(|m, state| {
//!             match *m.get_type() {
//!                 mecha::MessageType::Custom(INC) => { state.active },
//!                 _ => false
//!             }
//!         })
//!         .with_action(|_, state, _| {
//!             state.count += 1;
//!             println!("The new count is {}", state.count);
//!             Ok(())
//!         })
//!         // Specify activate match and action
//!         .with_match(|m, _| {
//!             match *m.get_type() {
//!                 mecha::MessageType::Custom(ACTIVATE) => true,
//!                 _ => false
//!             }
//!         })
//!         .with_action(|_, state, _| {
//!             state.active = true;
//!             println!("Actor activated!");
//!             Ok(())
//!         })
//!         // Go!
//!         .spawn_link(&initiator);
//!
//!     // Let's increment it three times.
//!     mecha::Message::custom(INC).send_to(&worker);
//!     mecha::Message::custom(INC).send_to(&worker);
//!     mecha::Message::custom(INC).send_to(&worker);
//!     thread::sleep(Duration::from_millis(500));
//!     // Nothing is really happening so far, we must also activate the actor.
//!     mecha::Message::custom(ACTIVATE).send_to(&worker);
//!     // Now things should be happening, and they should not be interrupted by
//!     // this shutdown message because there were still messages in the actor
//!     // process's mailbox and they are being processed before the shutdown.
//!     mecha::Message::shutdown().send_to(&worker);
//!
//!     // Now wait for the actor to send the Exited message back to us
//!     let msg = rx.recv().unwrap();
//!     assert_eq!(*msg.get_type(), mecha::MessageType::Exited);
//!     match *msg.get_datum() {
//!         mecha::MessageDatum::Void => { println!("Actor exited cleanly."); },
//!         _ => { println!("Actor must have exited with an error."); }
//!     }
//! }
//! ```
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!
//!

use std::thread;
use std::sync::mpsc;
use std::collections::HashMap;

extern crate uuid;

/// An ActorAddress structure is used, essentially, just as the identifier of an
/// actor for sending messages to it. ActorAddresses can be cheaply cloned and
/// passed around.
#[derive(Clone, Debug)]
pub struct ActorAddress {
    id: uuid::Uuid,
    endpoint: mpsc::Sender<Message>
}

impl ActorAddress {
    /// Creates a new ActorAddress with a provided sender half of a channel.
    pub fn new(endpoint: mpsc::Sender<Message>) -> ActorAddress {
        ActorAddress { id: uuid::Uuid::new_v4(), endpoint: endpoint }
    }
}

/// A MessageType defines a number of standard messages (such as the one to
/// stop an actor) and a Custom type which can be used to send user-defined
/// messages between actors.
///
/// Custom types contain a static str (so basically they're created from a
/// string literal) which can be matched on.
#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub enum MessageType {
    /// A message of this type notifies linked actors that the sender has
    /// exited. This message cannot be manually sent (and the builder pattern
    /// for Message prevents that); it is automatically sent to actors by the
    /// mecha implementation. If a message of this type has a Void datum, the
    /// actor exited normally; if it has a String datum, the string contains the
    /// exit reason (usually an error).
    Exited,
    /// A message of this type will tell the actor to be linked to the sender,
    /// i.e. the sender will be notified when the receiver exits.
    Link,
    /// A message of this type will stop and kill the actor receiving it.
    Shutdown,
    /// This is a custom message type to use for user-defined messages.
    Custom(&'static str),
}

/// This variant type specifies what kind of data can be passed around in
/// messages; we believe it is better to have a well defined variant type rather
/// than something like an Any. This makes serialization well defined, and
/// the Map variant can serialize complex data structures anyway.
#[derive(Clone, Debug)]
pub enum MessageDatum {
    Void,
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Map(HashMap<String, MessageDatum>),
    Act(ActorAddress)
}
impl MessageDatum {
    /// Extracts (clones) an i64 from the MessageDatum if possible.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            MessageDatum::I64(x) => Some(x),
            _ => None
        }
    }
    /// Extracts (clones) an u64 from the MessageDatum if possible.
    pub fn as_u64(&self) -> Option<u64> {
        match *self {
            MessageDatum::U64(x) => Some(x),
            _ => None
        }
    }
    /// Extracts (clones) an f64 from the MessageDatum if possible.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            MessageDatum::F64(x) => Some(x),
            _ => None
        }
    }
    /// Extracts (clones) a String from the MessageDatum if possible.
    pub fn as_str(&self) -> Option<String> {
        match *self {
            MessageDatum::Str(ref x) => Some(x.clone()),
            _ => None
        }
    }
    /// Extracts (clones) a Map from the MessageDatum if possible.
    pub fn as_map(&self) -> Option<HashMap<String, MessageDatum>> {
        match *self {
            MessageDatum::Map(ref m) => Some(m.clone()),
            _ => None
        }
    }
    /// Extracts (clones) an ActorAddress from the MessageDatum if possible.
    pub fn as_act(&self) -> Option<ActorAddress> {
        match *self {
            MessageDatum::Act(ref a) => Some(a.clone()),
            _ => None
        }
    }
}
impl From<i64> for MessageDatum {
    fn from(x: i64) -> MessageDatum { MessageDatum::I64(x) }
}
impl From<u64> for MessageDatum {
    fn from(x: u64) -> MessageDatum { MessageDatum::U64(x) }
}
impl From<f64> for MessageDatum {
    fn from(x: f64) -> MessageDatum { MessageDatum::F64(x) }
}
impl From<String> for MessageDatum {
    fn from(x: String) -> MessageDatum { MessageDatum::Str(x) }
}
impl<'a> From<&'a str> for MessageDatum {
    fn from(x: &'a str) -> MessageDatum { MessageDatum::Str(x.to_string()) }
}
impl From<HashMap<String, MessageDatum>> for MessageDatum {
    fn from(x: HashMap<String, MessageDatum>) -> MessageDatum { MessageDatum::Map(x) }
}
impl From<ActorAddress> for MessageDatum {
    fn from(x: ActorAddress) -> MessageDatum { MessageDatum::Act(x) }
}
impl<'a> From<&'a ActorAddress> for MessageDatum {
    fn from(x: &'a ActorAddress) -> MessageDatum { MessageDatum::Act(x.clone()) }
}

/// A Message contains a type, the actor from whom the message comes, and a
/// datum. A Message can be created and sent using a builder pattern.
#[derive(Clone, Debug)]
pub struct Message {
    mt: MessageType,
    sender: ActorAddress,
    datum: MessageDatum
}

/// The builder struct for a Message.
pub struct MessageBuilder {
    mt: MessageType,
    sender: Option<ActorAddress>,
    datum: Option<MessageDatum>
}

impl Message {
    /// Gets the type of the message.
    pub fn get_type(&self) -> &MessageType { &self.mt }
    /// Gets the sender of the message.
    pub fn get_sender(&self) -> &ActorAddress { &self.sender }
    /// Gets the datum of the message.
    pub fn get_datum(&self) -> &MessageDatum { &self.datum }

    /// Initializes a message builder for an Exited typed message.
    fn exited() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Exited,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a Link typed message.
    pub fn link() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Link,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a Shutdown typed message.
    pub fn shutdown() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Shutdown,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a Custom typed message.
    pub fn custom(mt: &'static str) -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Custom(mt),
            sender: None,
            datum: None,
        }
    }
}

impl MessageBuilder {
    /// Specifies the sender of the message.
    pub fn with_sender<'a>(&'a mut self, f: &ActorAddress) -> &'a mut MessageBuilder {
        self.sender = Some(f.clone());
        self
    }

    /// Specifies the datum of the message.
    pub fn with_datum<'a>(&'a mut self, d: MessageDatum) -> &'a mut MessageBuilder {
        self.datum = Some(d);
        self
    }

    /// Specifies an i64 as the datum of the message.
    pub fn with_i64<'a>(&'a mut self, i: i64) -> &'a mut MessageBuilder {
        self.with_datum(MessageDatum::from(i))
    }

    /// Specifies an u64 as the datum of the message.
    pub fn with_u64<'a>(&'a mut self, u: u64) -> &'a mut MessageBuilder {
        self.with_datum(MessageDatum::from(u))
    }

    /// Specifies an f64 as the datum of the message.
    pub fn with_f64<'a>(&'a mut self, f: f64) -> &'a mut MessageBuilder {
        self.with_datum(MessageDatum::from(f))
    }

    /// Specifies a string as the datum of the message.
    pub fn with_str<'a>(&'a mut self, s: &str) -> &'a mut MessageBuilder {
        self.with_datum(MessageDatum::from(s.to_string()))
    }

    /// Specifies a map as the datum of the message.
    pub fn with_map<'a>(&'a mut self, m: HashMap<String, MessageDatum>) -> &'a mut MessageBuilder {
        self.with_datum(MessageDatum::from(m))
    }

    /// Specifies an actor as the datum of the message.
    pub fn with_act<'a>(&'a mut self, a: &ActorAddress) -> &'a mut MessageBuilder {
        self.with_datum(MessageDatum::from(a))
    }

    /// Builds the Message, should a user want to store it. Generally this is
    /// not necessary, just use `send_to()` to send it directly.
    pub fn build(&self) -> Message {
        let (fake_tx, _) = std::sync::mpsc::channel();
        Message {
            mt: self.mt.clone(),
            sender: self.sender.clone().unwrap_or(
                ActorAddress::new(fake_tx)), // TODO: FIX. Leaks a broken channel.
            datum: match self.datum {
                None => MessageDatum::Void,
                Some(ref x) => x.clone(),
            }
        }
    }

    /// Builds the Message and sends it to the specified actor.
    ///
    /// This function is in the builder class for stylistic purposes: this
    /// way, it is possible to have nice one-liners.
    ///
    /// `mecha::Message::custom("blah").with_sender(&some_actor).with_i64(123).send_to(&other_actor);`
    ///
    pub fn send_to(&self, to: &ActorAddress) {
        to.endpoint.send(self.build()).unwrap_or(()); // TODO: Error handling.
    }
}

/// This is a utility struct you can use to specify that an actor is stateless.
#[derive(Default, Debug)]
pub struct Stateless;

type MatchResult = bool;
type ActionResult = Result<(), String>;

/// Actor provides an API for creating actor processes based on a definition of
/// state and a list of "match" clauses each with its own list of actions to
/// perform upon a match.
///
/// Actor uses a consuming builder pattern to collect all the data and functions
/// required before finally spawning off a process in another thread and
/// yielding an ActorAddress usable to send messages to the provided actor
/// process.
///
/// ```text
/// let address = mecha::Actor::new().with_state(SomeActorState{ ... })
///                                  .with_match( ... )
///                                  .with_action( ... )
///                                  .spawn();
/// ```
///
/// The actor state can be expressed by any struct or type as long as it is
/// sized, can be sent to another thread and can be defaulted.
///
/// Match clauses are functions or closures that take a reference to a Message
/// and a reference to the current actor state. They must return true only on a
/// successful match, and have no side effects.
///
/// Action clauses are functions or closures that take a reference to a Message
/// and a mutable reference to the current actor state (because it will be
/// potentially modified). They also take a reference to the address of the
/// actor process itself, so that it can be used as the sender of messages to
/// other actor processes.
pub struct Actor<ActorState: 'static + Sized + Default + Send> {
    state: ActorState,
    matches: Vec<Box<Fn(&Message, &ActorState) -> MatchResult + Send>>,
    actions: Vec<Vec<Box<Fn(&Message, &mut ActorState, &ActorAddress) -> ActionResult + Send>>>,
    mailbox: Vec<Message>,
    uplinks: Vec<ActorAddress>
}

impl<ActorState: 'static + Sized + Default + Send> Actor<ActorState> {

    /// Initializes the Actor building process.
    pub fn new() -> Self {
        Actor {
            state: ActorState::default(),
            matches: Vec::new(),
            actions: Vec::new(),
            mailbox: Vec::new(),
            uplinks: Vec::new(),
        }
    }

    /// Sets the initial state of the Actor. This method consumes the provided
    /// actor state.
    pub fn with_state(mut self, state: ActorState) -> Self {
        self.state = state;
        self
    }

    /// Adds a match clause to the Actor. Match clauses are functions or
    /// closures that take a reference to a Message and a reference to the
    /// current actor state. They must return true only on a successful match,
    /// and have no side effects.
    pub fn with_match<T>(mut self, mc: T) -> Self
        where T: 'static + Fn(&Message, &ActorState) -> MatchResult + Send {
        self.matches.push(Box::new(mc));
        self
    }

    /// Adds an action clause to the current match clause of the Actor. Action
    /// clauses are functions or closures that take a reference to a Message
    /// and a mutable reference to the current actor state (because it will be
    /// potentially modified). They also take a reference to the address of the
    /// actor process itself, so that it can be used as the sender of messages
    /// to other actor processes.
    pub fn with_action<T>(mut self, ac: T) -> Self
        where T: 'static + Fn(&Message, &mut ActorState, &ActorAddress) -> ActionResult + Send {
        while self.actions.len() < self.matches.len() {
            self.actions.push(Vec::new());
        }
        self.actions[self.matches.len() -1].push(Box::new(ac));
        self
    }

    /// Consumes the Actor building blocks and spawns the actor process,
    /// returning an ActorAddress for sending messages to it.
    pub fn spawn(self) -> ActorAddress {
        self.spawn_actor_loop(None)
    }

    /// Consumes the Actor building blocks and spawns the actor process,
    /// linking it to the provided actor (by its ActorAddress) and returning an
    /// ActorAddress for sending messages to the new actor process.
    pub fn spawn_link(self, uplink: &ActorAddress) -> ActorAddress {
        self.spawn_actor_loop(Some(uplink))
    }

    fn spawn_actor_loop(self, uplink: Option<&ActorAddress>) -> ActorAddress {
        let (tx, rx) = mpsc::channel::<Message>();
        let address = ActorAddress::new(tx.clone());
        let address_internal = address.clone();
        let mut actor = self;
        // Enqueue a Link message if we have an uplink.
        match uplink {
            None => (),
            Some(a) => Message::link().with_sender(a).send_to(&address_internal)
        }
        // Then spawn another thread.
        thread::spawn(move || {
            let receiver = rx;
            main_actor_loop(&mut actor, &address_internal, receiver);
        });
        address
    }
}

fn main_actor_loop<ActorState>(
    actor: &mut Actor<ActorState>,
    own_address: &ActorAddress,
    receiver: mpsc::Receiver<Message>)
    where ActorState: 'static + Sized + Default + Send {
    'main: loop {
        // Match zero or one message and perform the associated action.
        let mut matched = false;
        let mut result: Result<(), String> = Ok(());
        let mut matched_message_idx: usize = 0;
        'outer: for msg in actor.mailbox.iter() {
            for (matcher, actions) in actor.matches.iter().zip(actor.actions.iter()) {
                matched = matcher(msg, &actor.state);
                if matched {
                    for a in actions.iter() {
                        result = a(msg, &mut actor.state, &own_address);
                        match result {
                            Ok(()) => (),
                            Err(_) => { break 'outer; }
                        }
                    }
                    break 'outer;
                }
            }
            matched_message_idx += 1;
        }
        if matched {
            // If there was an error processing the message, we bail out
            // and notify the uplinks (the "let it crash" pattern).
            match result {
                Ok(()) => (),
                Err(ref e) => {
                    for u in actor.uplinks.iter() {
                        Message::exited().with_sender(&own_address)
                                         .with_str(e)
                                         .send_to(u);
                    }
                    break 'main;
                }
            }
            // If we've processed a message that would also trigger a
            // standard action, do it.
            let msg = actor.mailbox[matched_message_idx].clone();
            match *msg.get_type() {
                MessageType::Link => {
                    actor.uplinks.push(
                        msg.get_sender().clone());
                },
                MessageType::Shutdown => {
                    for u in actor.uplinks.iter() {
                        Message::exited()
                            .with_sender(&own_address)
                            .send_to(u);
                    }
                    break 'main;
                },
                _ => ()
            }
            // Now remove the matched message as we've processed it, and
            // continue without pulling from the queue.
            // TODO: currently very inefficient, we can do better.
            actor.mailbox.remove(matched_message_idx);
            continue;
        }

        // We could still be in a situation where the client hasn't
        // matched a standard message, but we have some in the mailbox.
        // Loop again and do some standard matching.
        matched = false; matched_message_idx = 0;
        for msg in actor.mailbox.iter() {
            match *msg.get_type() {
                MessageType::Link => {
                    actor.uplinks.push(
                        msg.get_sender().clone());
                    matched = true;
                    break;
                },
                MessageType::Shutdown => {
                    for u in actor.uplinks.iter() {
                        Message::exited()
                            .with_sender(&own_address)
                            .send_to(u);
                    }
                    break 'main;
                },
                _ => (),
            }
            matched_message_idx += 1;
        }
        if matched {
            actor.mailbox.remove(matched_message_idx);
            continue;
        }

        // If we get here, the mailbox is either empty or nothing can be
        // matched, so wait for another message.
        let rcvd_msg = receiver.recv();
        match rcvd_msg {
            Ok(msg) => {
                actor.mailbox.push(msg);
            },
            Err(_) => {
                panic!("Exiting upon error receiving from a channel!");
            }
        }
    }

}





#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_c5d1;

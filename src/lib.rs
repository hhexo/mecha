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
    /// A message of this type is automatically sent to any actor upon spawning.
    /// If an actor does not need initialization, it can safely ignore the
    /// message. This message cannot be manually sent (and the builder pattern
    /// for Message prevents that); it is automatically sent to actors by the
    /// mecha implementation.
    Init,
    /// A message of this type notifies linked actors that the sender has
    /// exited. This message cannot be manually sent (and the builder pattern
    /// for Message prevents that); it is automatically sent to actors by the
    /// mecha implementation.
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
    /// Consumes the MessageDatum and converts it to i64 if possible.
    pub fn as_i64(self) -> Option<i64> {
        match self {
            MessageDatum::I64(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageDatum and converts it to u64 if possible.
    pub fn as_u64(self) -> Option<u64> {
        match self {
            MessageDatum::U64(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageDatum and converts it to f64 if possible.
    pub fn as_f64(self) -> Option<f64> {
        match self {
            MessageDatum::F64(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageDatum and converts it to String if possible.
    pub fn as_str(self) -> Option<String> {
        match self {
            MessageDatum::Str(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageDatum and converts it to a map if possible.
    pub fn as_map(self) -> Option<HashMap<String, MessageDatum>> {
        match self {
            MessageDatum::Map(m) => Some(m),
            _ => None
        }
    }
    /// Consumes the MessageDatum and converts it to an ActorAddress if
    /// possible.
    pub fn as_act(self) -> Option<ActorAddress> {
        match self {
            MessageDatum::Act(a) => Some(a),
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

    /// Initializes a message builder for an Init typed message.
    fn init() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Init,
            sender: None,
            datum: None,
        }
    }

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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_c5d1;

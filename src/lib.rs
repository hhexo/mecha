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
//! # Actors and messages
//!
//! Here is a quick example of how to spawn an actor and send a message to it.
//!
//! ```
//! extern crate mecha;
//!
//! struct ActorImplementation;
//!
//! impl mecha::Actor for ActorImplementation {
//!     fn process_message(&mut self,
//!                        message: mecha::Message,
//!                        myself: &mecha::ActorAddress) {
//!         println!("I received a message! It was of type...");
//!         println!("    {:?}", message.get_type());
//!         println!("...with datum...");
//!         println!("    {:?}", message.get_datum());
//!     }
//! }
//!
//! fn main() {
//!     let actor = mecha::spawn(ActorImplementation);
//!
//!     mecha::Message::custom("MyMessage")
//!         .with_datum(mecha::MessageDatum::from("blah"))
//!         .send_to(&actor);
//!
//!     // Or, more simply:
//!
//!     mecha::Message::custom("MyMessage").with_str("blah").send_to(&actor);
//!
//!     // Be clean and don't forget to stop the actor at the end!
//!
//!     mecha::Message::shutdown().send_to(&actor);
//! }
//! ```
//!
//! Generally you will then have to wait until the actor (which is running in
//! another thread) has finished processing the messages and printed the output.
//! For a quick-and-dirty example you can just wait a bit:
//!
//! ```
//!     use std::thread;
//!     use std::time::Duration;
//!     thread::sleep(Duration::from_millis(500));
//! ```
//!
//! However, that is not the best way to do it. You can create a channel and
//! wrap the sender part of it in an ActorAddress; then you can use `spawn_link`
//! instead, so your fake actor will be notified when the actor shuts down. At
//! that point you can use the receiver part of the channel to wait for the
//! Exited message.
//!
//! The following is the preferred pattern to use:
//!
//! ```
//! struct ActorImplementation;
//!
//! impl mecha::Actor for ActorImplementation {
//!     fn process_message(&mut self,
//!                        message: mecha::Message,
//!                        myself: &mecha::ActorAddress) {
//!         println!("I received a message! It was of type...");
//!         println!("    {:?}", message.get_type());
//!         println!("...with datum...");
//!         println!("    {:?}", message.get_datum());
//!     }
//! }
//!
//! fn main() {
//!     let (tx, rx) = std::sync::mpsc::channel();
//!     let initiator = mecha::ActorAddress::new(tx);
//!     let actor = mecha::spawn_link(ActorImplementation, &initiator);
//!
//!     mecha::Message::custom("MyMessage").with_str("blah").send_to(&actor);
//!
//!     mecha::Message::shutdown().send_to(&actor);
//!
//!     let m = rx.recv().unwrap();
//!     assert!(*m.get_type() == mecha::MessageType::Exited);
//! }
//! ```
//! You can even have an actor spawn another and link to it during its
//! initialization. For example, an actor implementation could look like this:
//!
//! ```
//! struct ActorImplementation;
//!
//! impl mecha::Actor for ActorImplementation {
//!     fn process_message(&mut self,
//!                        message: mecha::Message,
//!                        myself: &mecha::ActorAddress) {
//!         println!("I received a message! It was of type...");
//!         println!("    {:?}", message.get_type());
//!         println!("...with datum...");
//!         println!("    {:?}", message.get_datum());
//!     }
//! }
//!
//! struct InitiatorActorImplementation;
//!
//! impl mecha::Actor for InitiatorActorImplementation {
//!     fn process_message(&mut self,
//!                        message: mecha::Message,
//!                        myself: &mecha::ActorAddress) {
//!         match *message.get_type() {
//!             mecha::MessageType::Init => {
//!                 mecha::spawn_link(ActorImplementation, myself);
//!             },
//!             mecha::MessageType::Exited => {
//!                 // The other actor has finished, so we can send ourselves
//!                 // a Shutdown message.
//!                 mecha::Message::shutdown().send_to(myself);
//!             },
//!             _ => ()
//!         }
//!     }
//! }
//! ```
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
    /// This message is used to register an actor with the MasterControlProgram
    /// actor API. This message cannot be manually sent (and the builder
    /// pattern for Message prevents that); it is sent to an MCP instance by the
    /// MCP API implementation.
    ///
    /// Register implies Link in the opposite direction. This means that if A
    /// sends a Register message to the MCP, then the MCP will immediately send
    /// a Link message to A.
    Register,
    /// This message is sent back as a response from the MasterControlProgram
    /// when a Register message is received. This message cannot be manually
    /// sent (and the builder pattern for Message prevents that); it is only
    /// sent by an MCP instance.
    RegisterResponse,
    /// This message is used to request an actor by name from the
    /// MasterControlProgram actor API. This message cannot be manually sent
    /// (and the builder pattern for Message prevents that); it is sent to an
    /// MCP instance by the MCP API implementation.
    WhereIs,
    /// This message is the response for the request of an actor by name from
    /// the MasterControlProgram actor API. This message cannot be manually sent
    /// (and the builder pattern for Message prevents that); it is only sent by
    /// an MCP instance and handled by the MCP API.
    WhereIsResponse,
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

    /// Initializes a message builder for a Register typed message.
    fn register() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Register,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a RegisterResponse typed message.
    fn register_response() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::RegisterResponse,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a WhereIs typed message.
    fn where_is() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::WhereIs,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a WhereIsResponse typed message.
    fn where_is_response() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::WhereIsResponse,
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

/// This trait represents the interface of any actor implementation.
///
/// Implementing this trait effectively requires to implement the functionality
/// to act upon one single message. The `mecha` library takes care of handling
/// the mailbox and the loop which receives messages.
pub trait Actor {
    /// This function is the meat of the message processing and must be
    /// implemented to implement this trait.
    fn process_message(&mut self, message: Message, myself: &ActorAddress);
}

#[derive(Debug)]
struct ActorInternal {
    address: ActorAddress,
    mailbox: mpsc::Receiver<Message>,
    uplinks: Vec<ActorAddress>,
}

fn spawn_internal<T: Actor + Send + 'static>(mut implementor: T,
                                             uplink: Option<&ActorAddress>) -> ActorAddress {
    let (tx, rx) = mpsc::channel::<Message>();
    let actor = ActorAddress::new(tx.clone());
    let actor_internal = actor.clone();
    // Put an Init message on the channel immediately.
    Message::init().send_to(&actor);
    // Also put a Link message if we have an uplink.
    match uplink {
        None => (),
        Some(a) => Message::link().with_sender(a).send_to(&actor)
    }
    // Then spawn another thread.
    thread::spawn(move || {
        let mut internal = ActorInternal {
            address: actor_internal,
            mailbox: rx,
            uplinks: Vec::new(),
        };
        loop {
            let rcvd_msg = internal.mailbox.recv();
            match rcvd_msg {
                Ok(msg) => {
                    let mt = msg.get_type().clone(); // for later
                    let ms = msg.get_sender().clone(); // for later
                    // Consume and process the message
                    implementor.process_message(msg, &internal.address);
                    // Perform extra standard actions
                    match mt {
                        MessageType::Link => {
                            // Link this actor to the sender.
                            internal.uplinks.push(ms);
                        },
                        MessageType::Shutdown => {
                            // Shut down this actor and send a message to all
                            // uplinks. The string value is the name of the
                            // actor (for deregistering purposes).
                            for a in internal.uplinks.iter() {
                                Message::exited().with_sender(&internal.address)
                                                 .send_to(a);
                            }
                            break;
                        },
                        _ => ()
                    }
                },
                Err(_) => {
                    panic!("Exiting upon error receiving from a channel!");
                }
            }
        }
    });
    actor
}


/// Spawns an actor based on the provided Actor implementor, whose ownership is
/// acquired by the spawned thread. Returns an ActorAddress identifying the
/// spawned actor.
pub fn spawn<T: Actor + Send + 'static>(implementor: T) -> ActorAddress {
    spawn_internal(implementor, None)
}

/// Spawns an actor based on the provided Actor implementor, whose ownership is
/// acquired by the spawned thread, and links it to the actor provided (this
/// means the actor provided will be notified when the new actor exits).
/// Returns an ActorAddress identifying the spawned actor.
pub fn spawn_link<T: Actor + Send + 'static>(implementor: T, uplink: &ActorAddress) -> ActorAddress {
    spawn_internal(implementor, Some(uplink))
}



#[derive(Debug)]
struct MasterControlProgramImpl {
    registered: HashMap<String, ActorAddress>,
    lookup: HashMap<uuid::Uuid, String>
}

const MCP_REGISTER_NAME_KEY: &'static str = "name";
const MCP_REGISTER_ACTOR_KEY: &'static str = "actor";

impl Actor for MasterControlProgramImpl {
    fn process_message(&mut self, message: Message, myself: &ActorAddress) {
        match *message.get_type() {
            MessageType::Exited => {
                let id = message.get_sender().id.clone();
                let already_there = self.lookup.contains_key(&id);
                if already_there {
                    let key = self.lookup.get(&id).unwrap().clone();
                    self.registered.remove(&key);
                    self.lookup.remove(&id);
                }
            },
            MessageType::Register => {
                let data = message.get_datum().clone().as_map();
                match data {
                    None => {
                        // Void datum means registration failed.
                        Message::register_response()
                            .with_sender(myself)
                            .send_to(message.get_sender());
                    },
                    Some(ref m) => {
                        let name = m.get(MCP_REGISTER_NAME_KEY).unwrap().clone().as_str().unwrap();
                        let actor = m.get(MCP_REGISTER_ACTOR_KEY).unwrap().clone().as_act().unwrap();
                        let already_there = self.registered.contains_key(&name);
                        if already_there {
                            // Void datum means registration failed.
                            Message::register_response()
                                .with_sender(myself)
                                .send_to(message.get_sender());
                        } else {
                            self.registered.insert(name.clone(),
                                                   actor.clone());
                            self.lookup.insert(actor.id.clone(),
                                               name.clone());
                            Message::link().with_sender(myself)
                                           .send_to(&actor);
                            // If registration succeeds we send back the name.
                            Message::register_response()
                                .with_sender(myself)
                                .with_str(&name)
                                .send_to(message.get_sender());
                        }

                    }
                }
            },
            MessageType::WhereIs => {
                match message.get_datum().clone().as_str() {
                    None => {
                        // Void datum means "no actor found".
                        Message::where_is_response()
                            .with_sender(myself)
                            .send_to(message.get_sender());
                    },
                    Some(s) => {
                        let already_there = self.registered.contains_key(&s);
                        if !already_there {
                            // Void datum means "no actor found".
                            Message::where_is_response()
                                .with_sender(myself)
                                .send_to(message.get_sender());
                        } else {
                            Message::where_is_response()
                                .with_sender(myself)
                                .with_act(self.registered.get(&s).unwrap())
                                .send_to(message.get_sender());
                        }
                    }
                }
            },
            MessageType::Shutdown => {
                // This will cause the Exited messages to be dropped off a
                // broken channel. Oh well.
                for (_, v) in self.registered.iter() {
                    Message::shutdown().with_sender(myself).send_to(v);
                }
            },
            _ => ()
        }
    }
}


/// The Master Control Program (yes, it's a TRON reference) is essentially the
/// system process (it's a sort of equivalent to Process in Elixir). It's
/// internally implemented as an actor, but all its APIs are synchronous.
///
/// It offers the functionality to register and get actors by name. When an
/// actor is registered, it is also linked with the MCP so it knows when it
/// disappears.
///
/// If the MasterControlProgram is dropped, its actor is shut down and all
/// registered actors will also be shut down, in undefined order. That is not
/// the recommended or best way to tear down the system.
#[derive(Debug)]
pub struct MasterControlProgram {
    actor: ActorAddress
}

impl MasterControlProgram {
    /// Creates a new MasterControlProgram. Normally you'll only need one.
    pub fn new() -> MasterControlProgram {
        MasterControlProgram {
            actor: spawn(MasterControlProgramImpl {
                registered: HashMap::new(),
                lookup: HashMap::new()
            })
        }
    }

    /// Registers an actor with the MasterControlProgram. If the registration
    /// succeeds, the actor will be linked to the MasterControlProgram actor and
    /// true is returned; if it fails, false is returned.
    pub fn register(&self, name: &str, actor: &ActorAddress) -> bool{
        let (tx, rx) = mpsc::channel();
        let temp_actor = ActorAddress::new(tx);
        let mut data = HashMap::new();
        data.insert(MCP_REGISTER_NAME_KEY.to_string(), MessageDatum::from(name));
        data.insert(MCP_REGISTER_ACTOR_KEY.to_string(), MessageDatum::from(actor));
        Message::register().with_sender(&temp_actor)
                           .with_map(data)
                           .send_to(&self.actor);
        let m = rx.recv().unwrap();
        match *m.get_type() {
            MessageType::RegisterResponse => {
                match *m.get_datum() {
                    MessageDatum::Str(_) => true,
                    _ => false
                }
            },
            _ => {
                // Something's wrong
                false
            }
        }
    }

    /// Queries the MasterControlProgram for an actor by name. If no actor is
    /// present with the specified name, None will be returned. This API is
    /// synchronous.
    pub fn where_is(&self, name: &str) -> Option<ActorAddress> {
        let (tx, rx) = mpsc::channel();
        let temp_actor = ActorAddress::new(tx);
        Message::where_is().with_sender(&temp_actor)
                           .with_str(name)
                           .send_to(&self.actor);
        let m = rx.recv().unwrap();
        match *m.get_type() {
            MessageType::WhereIsResponse => {
                match *m.get_datum() {
                    MessageDatum::Act(ref a) => { Some(a.clone()) },
                    _ => None
                }
            },
            _ => {
                // Something's wrong
                None
            }
        }
    }

    /// Tears down the MCP. Use the usual trick to make it synchronous.
    fn tear_down(&self) {
        let (tx, rx) = mpsc::channel();
        let temp_actor = ActorAddress::new(tx);
        Message::link().with_sender(&temp_actor).send_to(&self.actor);
        Message::shutdown().send_to(&self.actor);
        let m = rx.recv().unwrap();
        assert!(*m.get_type() == MessageType::Exited);
    }
}

impl Drop for MasterControlProgram {
    fn drop(&mut self) {
        self.tear_down();
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_c5d1;

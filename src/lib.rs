use std::thread;
use std::sync::mpsc;
use std::collections::HashMap;

/// An ActorAddress structure is used, essentially, just as the identifier of an
/// actor for sending messages to it. ActorAddresses can be cheaply cloned and
/// passed around.
#[derive(Clone, Debug)]
pub struct ActorAddress {
    endpoint: mpsc::Sender<Message>
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
    /// message.
    Init,
    /// A message of this type will stop and kill the actor receiving it.
    Stop,
    /// This is a custom message type to use for user-defined messages.
    Custom(&'static str)
}

/// This variant type specifies what kind of data can be passed around in
/// messages; we believe it is better to have a well defined variant type rather
/// than something like a Box<Any>. This makes serialization well defined, and
/// the Map variant can serialize complex data structures.
#[derive(Clone, Debug)]
pub enum MessageVariant {
    Void,
    I64(i64),
    U64(u64),
    F64(f64),
    Str(String),
    Map(HashMap<String, MessageVariant>),
    Act(ActorAddress)
}
impl MessageVariant {
    /// Consumes the MessageVariant and converts it to i64 if possible.
    fn as_i64(self) -> Option<i64> {
        match self {
            MessageVariant::I64(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageVariant and converts it to u64 if possible.
    fn as_u64(self) -> Option<u64> {
        match self {
            MessageVariant::U64(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageVariant and converts it to f64 if possible.
    fn as_f64(self) -> Option<f64> {
        match self {
            MessageVariant::F64(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageVariant and converts it to String if possible.
    fn as_str(self) -> Option<String> {
        match self {
            MessageVariant::Str(x) => Some(x),
            _ => None
        }
    }
    /// Consumes the MessageVariant and converts it to a map if possible.
    fn as_map(self) -> Option<HashMap<String, MessageVariant>> {
        match self {
            MessageVariant::Map(m) => Some(m),
            _ => None
        }
    }
    /// Consumes the MessageVariant and converts it to an ActorAddress if
    /// possible.
    fn as_act(self) -> Option<ActorAddress> {
        match self {
            MessageVariant::Act(a) => Some(a),
            _ => None
        }
    }
}
impl From<i64> for MessageVariant {
    fn from(x: i64) -> MessageVariant { MessageVariant::I64(x) }
}
impl From<u64> for MessageVariant {
    fn from(x: u64) -> MessageVariant { MessageVariant::U64(x) }
}
impl From<f64> for MessageVariant {
    fn from(x: f64) -> MessageVariant { MessageVariant::F64(x) }
}
impl From<String> for MessageVariant {
    fn from(x: String) -> MessageVariant { MessageVariant::Str(x) }
}
impl<'a> From<&'a str> for MessageVariant {
    fn from(x: &'a str) -> MessageVariant { MessageVariant::Str(x.to_string()) }
}
impl From<HashMap<String, MessageVariant>> for MessageVariant {
    fn from(x: HashMap<String, MessageVariant>) -> MessageVariant { MessageVariant::Map(x) }
}
impl From<ActorAddress> for MessageVariant {
    fn from(x: ActorAddress) -> MessageVariant { MessageVariant::Act(x) }
}
impl<'a> From<&'a ActorAddress> for MessageVariant {
    fn from(x: &'a ActorAddress) -> MessageVariant { MessageVariant::Act(x.clone()) }
}

/// A Message contains a type, the actor from whom the message comes, and a
/// datum. A Message can be created and sent using a builder pattern.
#[derive(Clone, Debug)]
pub struct Message {
    mt: MessageType,
    sender: ActorAddress,
    datum: MessageVariant
}

/// The builder struct for a Message.
pub struct MessageBuilder {
    mt: MessageType,
    sender: Option<ActorAddress>,
    datum: Option<MessageVariant>
}

impl Message {
    /// Gets the type of the message.
    pub fn get_type(&self) -> &MessageType { &self.mt }
    /// Gets the sender of the message.
    pub fn get_sender(&self) -> &ActorAddress { &self.sender }
    /// Gets the datum of the message.
    pub fn get_datum(&self) -> &MessageVariant { &self.datum }

    /// Initializes a message builder for an Init typed message.
    pub fn init() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Init,
            sender: None,
            datum: None,
        }
    }

    /// Initializes a message builder for a Stop typed message.
    pub fn stop() -> MessageBuilder {
        MessageBuilder {
            mt: MessageType::Stop,
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
    pub fn with_datum<'a>(&'a mut self, d: MessageVariant) -> &'a mut MessageBuilder {
        self.datum = Some(d);
        self
    }

    /// Specifies an i64 as the datum of the message.
    pub fn with_i64<'a>(&'a mut self, i: i64) -> &'a mut MessageBuilder {
        self.with_datum(MessageVariant::from(i))
    }

    /// Specifies an u64 as the datum of the message.
    pub fn with_u64<'a>(&'a mut self, u: u64) -> &'a mut MessageBuilder {
        self.with_datum(MessageVariant::from(u))
    }

    /// Specifies an f64 as the datum of the message.
    pub fn with_f64<'a>(&'a mut self, f: f64) -> &'a mut MessageBuilder {
        self.with_datum(MessageVariant::from(f))
    }

    /// Specifies a string as the datum of the message.
    pub fn with_str<'a>(&'a mut self, s: &str) -> &'a mut MessageBuilder {
        self.with_datum(MessageVariant::from(s.to_string()))
    }

    /// Specifies a map as the datum of the message.
    pub fn with_map<'a>(&'a mut self, m: HashMap<String, MessageVariant>) -> &'a mut MessageBuilder {
        self.with_datum(MessageVariant::from(m))
    }

    /// Specifies an actor as the datum of the message.
    pub fn with_act<'a>(&'a mut self, a: &ActorAddress) -> &'a mut MessageBuilder {
        self.with_datum(MessageVariant::from(a))
    }

    fn build(&self) -> Message {
        let (fake_tx, _) = std::sync::mpsc::channel();
        Message {
            mt: self.mt.clone(),
            sender: self.sender.clone().unwrap_or(
                ActorAddress { endpoint: fake_tx }), // TODO: FIX. Leaks a broken channel.
            datum: match self.datum {
                None => MessageVariant::Void,
                Some(ref x) => x.clone(),
            }
        }
    }

    /// Builds the Message and sends it to the specified actor.
    ///
    /// This function is in the builder class for stylistic purposes: this
    /// way, it is possible to have nice one-liners.
    ///
    /// `Message::custom("blah").sender(&some_actor).with_i64(123).send_to(&other_actor);`
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

/// Spawns an actor based on the provided Actor implementor, whose ownership is
/// acquired by the spawned thread. Returns an ActorAddress identifying the
/// spawned actor.
pub fn spawn<T: Actor + Send + 'static>(mut implementor: T) -> ActorAddress {
    let (tx, rx) = mpsc::channel::<Message>();
    let actor = ActorAddress { endpoint: tx.clone() };
    // Put an Init message on the channel immediately.
    Message::init().send_to(&actor);
    // Then spawn another thread.
    thread::spawn(move || {
        let rx_internal = rx;
        let tx_internal = tx;
        let actor_internal = ActorAddress { endpoint: tx_internal };
        loop {
            let rcvd_msg = rx_internal.recv();
            match rcvd_msg {
                Ok(msg) => {
                    let mt = msg.get_type().clone(); // for later
                    // Consume and process the message
                    implementor.process_message(msg, &actor_internal);
                    // Perform extra standard actions
                    match mt {
                        MessageType::Stop => {
                            // Stop this actor.
                            println!("Exiting upon request.");
                            break;
                        },
                        _ => ()
                    }
                },
                Err(_) => {
                    println!("Exiting upon error receiving message!");
                    break;
                }
            }
        }
    });
    actor
}


#[cfg(test)]
mod tests;

# mecha

Mecha is yet another actor model implementation written in Rust.

There are already Rust crates implementing such model. I'll give a shout out
to [RobotS](https://github.com/gamazeps/RobotS) and
[acto-rs](https://github.com/dbeck/acto-rs), but I suspect there are others,
and some building blocks are there in other libraries.

The main reason why I am writing this library is as a learning exercise, and
it should not be expected to be feature-complete (I wish!) or stable.

Do not use this in production code.

# Using mecha

If you really want to use mecha, feel free to add a git dependency. I
haven't gone through the effort of uploading a crate to crates.io yet.

`mecha = { git = "https://github.com/hhexo/mecha.git" }`

# Actors and messages

Here is a quick example of how to spawn an actor and send a message to it.

```
extern crate mecha;

struct ActorImplementation;

impl mecha::Actor for ActorImplementation {
    fn process_message(&mut self,
                       message: mecha::Message,
                       myself: &mecha::ActorAddress) {
        println!("I received a message! It was of type...");
        println!("    {:?}", message.get_type());
        println!("...with datum...");
        println!("    {:?}", message.get_datum());
    }
}

fn main() {
    let actor = mecha::spawn(ActorImplementation);

    mecha::Message::custom("MyMessage")
        .with_datum(mecha::MessageDatum::from("blah"))
        .send_to(&actor);

    // Or, more simply:

    mecha::Message::custom("MyMessage").with_str("blah").send_to(&actor);

    // Be clean and don't forget to stop the actor at the end!

    mecha::Message::shutdown().send_to(&actor);
}
```

Generally you will then have to wait until the actor (which is running in
another thread) has finished processing the messages and printed the output.
For a quick-and-dirty example you can just wait a bit:

```
    use std::thread;
    use std::time::Duration;
    thread::sleep(Duration::from_millis(500));
```

However, that is not the best way to do it. You can create a channel and
wrap the sender part of it in an ActorAddress; then you can use `spawn_link`
instead, so your fake actor will be notified when the actor shuts down. At
that point you can use the receiver part of the channel to wait for the
Exited message.

The following is the preferred pattern to use:

```
struct ActorImplementation;

impl mecha::Actor for ActorImplementation {
    fn process_message(&mut self,
                       message: mecha::Message,
                       myself: &mecha::ActorAddress) {
        println!("I received a message! It was of type...");
        println!("    {:?}", message.get_type());
        println!("...with datum...");
        println!("    {:?}", message.get_datum());
    }
}

fn main() {
    let (tx, rx) = std::sync::mpsc::channel();
    let initiator = mecha::ActorAddress::new(tx);
    let actor = mecha::spawn_link(ActorImplementation, &initiator);

    mecha::Message::custom("MyMessage").with_str("blah").send_to(&actor);

    mecha::Message::shutdown().send_to(&actor);

    let m = rx.recv().unwrap();
    assert!(*m.get_type() == mecha::MessageType::Exited);
}
```
You can even have an actor spawn another and link to it during its
initialization. For example, an actor implementation could look like this:

```
struct ActorImplementation;

impl mecha::Actor for ActorImplementation {
    fn process_message(&mut self,
                       message: mecha::Message,
                       myself: &mecha::ActorAddress) {
        println!("I received a message! It was of type...");
        println!("    {:?}", message.get_type());
        println!("...with datum...");
        println!("    {:?}", message.get_datum());
    }
}

struct InitiatorActorImplementation;

impl mecha::Actor for InitiatorActorImplementation {
    fn process_message(&mut self,
                       message: mecha::Message,
                       myself: &mecha::ActorAddress) {
        match *message.get_type() {
            mecha::MessageType::Init => {
                mecha::spawn_link(ActorImplementation, myself);
            },
            mecha::MessageType::Exited => {
                // The other actor has finished, so we can send ourselves
                // a Shutdown message.
                mecha::Message::shutdown().send_to(myself);
            },
            _ => ()
        }
    }
}
```

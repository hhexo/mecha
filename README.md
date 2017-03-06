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

# Examples

## The simple example

Here is a quick and dirty example of how to spawn an actor and send
messages to it.

```
extern crate mecha;

fn main() {
    let worker = mecha::Actor::new().with_state(mecha::Stateless)
        .with_match(|msg, _| {
            match *msg.get_type() {
                mecha::MessageType::Custom(_) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("{:?}", msg);
            Ok(())
        })
        .spawn();

    // Void message
    mecha::Message::custom("blah").send_to(&worker);
    // String message
    mecha::Message::custom("blah").with_str("Hello!").send_to(&worker);
    // Shut down the worker actor
    mecha::Message::shutdown().send_to(&worker);

    // Wait a bit
    use std::thread;
    use std::time::Duration;
    thread::sleep(Duration::from_millis(500));
}
```

Generally you will want to wait until the actor process (which is running in
another thread) has finished processing the messages and printed the output.

For this quick and dirty example we have just waited a bit. However, that is
not the correct way to do it. You really want to wait until you know for
certain that the actor process has shut down.

## Linking actors

Actor processes can be _linked_ to each other. If A is linked to B, then B
will send A an `Exited` message whenever it exits due to either a shutdown
request (in which case the datum of the message is `Void`) or an error (in
which case the datum of the message is a `String` containing an error
reason).

Your main program is not an actor process, however. How can you link it to
the actor process you have spawned?

You can create a Rust `mpsc` channel and wrap the sender part of it in an
ActorAddress to create a "fake" actor address. Then you can use `spawn_link`
instead of `spawn`, so your fake actor will be immediately linked to the
spawned worker actor. Once the worker exits, an `Exited` message will be
available at the receiving end of the `mpsc` channel, therefore you can wait
for it.

The following is an example of such pattern:

```
extern crate mecha;
use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel();
    let initiator = mecha::ActorAddress::new(tx);

    let worker = mecha::Actor::new().with_state(mecha::Stateless)
        .with_match(|msg, _| {
            match *msg.get_type() {
                mecha::MessageType::Custom(_) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("{:?}", msg);
            Ok(())
        })
        .spawn_link(&initiator);

    // Void message
    mecha::Message::custom("blah").send_to(&worker);
    // String message
    mecha::Message::custom("blah").with_str("Hello!").send_to(&worker);
    // Shut down the worker actor
    mecha::Message::shutdown().send_to(&worker);

    // Now wait for the actor to send the Exited message back to us
    let msg = rx.recv().unwrap();
    assert_eq!(*msg.get_type(), mecha::MessageType::Exited);
    match *msg.get_datum() {
        mecha::MessageDatum::Void => { println!("Actor exited cleanly."); },
        _ => { println!("Actor must have exited with an error."); }
    }
}
```

It is also possible to link to an actor _after_ it has been spawned. This is
done by sending a `Link` message to an existing actor process.

```
extern crate mecha;
use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel();
    let initiator = mecha::ActorAddress::new(tx);

    let worker = mecha::Actor::new().with_state(mecha::Stateless)
        .with_match(|msg, _| {
            match *msg.get_type() {
                mecha::MessageType::Custom(_) => true,
                _ => false
            }
        })
        .with_action(|msg, _, _| {
            println!("{:?}", msg);
            Ok(())
        })
        .spawn();

    // Link after spawn
    mecha::Message::link().with_sender(&initiator).send_to(&worker);

    // ...
}
```

In fact, the `spawn_link` function does nothing more than spawning an actor
process and immediately sending a `Link` message to it.

## Stateful actors

In the previous examples, the actor process was stateless; however it is
possible to have stateful actors so that match clauses can make informed
decisions based on state, and actions can modify the actor's state.

Here is an example of a counter which is sensitive to two messages: one that
increments it only if the counter is "active", and the other one that
activates the actor.

```
extern crate mecha;
use std::sync::mpsc;

#[derive(Default)]
struct CounterState { active: bool, count: i32 }

const INC : &'static str = ":inc";
const ACTIVATE : &'static str = ":activate";

#[test]
fn test_stateful() {
    let (tx, rx) = mpsc::channel();
    let initiator = mecha::ActorAddress::new(tx);

    let worker = mecha::Actor::new()
        // Initial state
        .with_state(CounterState {
            active: false,
            count: 0
        })
        // Specify increment match and action
        .with_match(|m, state| {
            match *m.get_type() {
                mecha::MessageType::Custom(INC) => { state.active },
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
                mecha::MessageType::Custom(ACTIVATE) => true,
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
    mecha::Message::custom(INC).send_to(&worker);
    mecha::Message::custom(INC).send_to(&worker);
    mecha::Message::custom(INC).send_to(&worker);
    thread::sleep(Duration::from_millis(500));
    // Nothing is really happening so far, we must also activate the actor.
    mecha::Message::custom(ACTIVATE).send_to(&worker);
    // Now things should be happening, and they should not be interrupted by
    // this shutdown message because there were still messages in the actor
    // process's mailbox and they are being processed before the shutdown.
    mecha::Message::shutdown().send_to(&worker);

    // Now wait for the actor to send the Exited message back to us
    let msg = rx.recv().unwrap();
    assert_eq!(*msg.get_type(), mecha::MessageType::Exited);
    match *msg.get_datum() {
        mecha::MessageDatum::Void => { println!("Actor exited cleanly."); },
        _ => { println!("Actor must have exited with an error."); }
    }
}
```

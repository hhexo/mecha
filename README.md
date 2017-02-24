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

let actor = mecha::spawn(ActorImplementation);

mecha::Message::custom("MyMessage")
    .with_datum(mecha::MessageVariant::from("blah"))
    .send_to(&actor);

// Be clean and don't forget to stop the actor at the end!

mecha::Message::stop().send_to(&actor);

```

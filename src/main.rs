#![feature(conservative_impl_trait)]
#[macro_use]
extern crate error_chain;
extern crate irc;
extern crate mysql;

mod errors {
    error_chain!{}
}
mod sasl;
mod msg_handler;

pub use errors::*;
use irc::client::prelude::*;
use irc::proto::command::Command::*;
use mysql::{OptsBuilder, Pool};

fn main() {
    let server = IrcServer::new("config.toml").unwrap();
    let mut opts_builder = OptsBuilder::new();
    opts_builder
        .socket(Some(server.config().get_option("mysql_socket")))
        .user(Some(server.config().get_option("mysql_user")))
        .pass(Some(server.config().get_option("mysql_password")))
        .db_name(Some(server.config().get_option("mysql_db")));
    let pool = Pool::new(opts_builder).unwrap();
    let stream = sasl::auth(&server, server.stream()).expect("SASL Authentication failed");

    //println!("{:#?}", pool.prep_exec(r#"SELECT * FROM test"#, Params::Empty).unwrap().last());

    for msg in stream
        .filter(|msg| match msg.command {
            Command::PRIVMSG(_, ref msg) => msg.starts_with("!"),
            _ => false,
        })
        .wait()
    {
        let _ = msg.map(|msg| {
            let msg2 = msg.clone();
            match msg.command {
            PRIVMSG(_, mut msg_text) => {
                msg_text.remove(0);
                msg_handler::handle_command(&server, msg2, msg_text)
            },
            _ => unreachable!(),
        }});
    }
}

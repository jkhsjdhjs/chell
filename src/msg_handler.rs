use irc::client::prelude::*;

enum Command {
    DAILY1337,
    GUESS(String),
    NONE
}

fn get_command(msg_text: String) -> Command {
    use self::Command::*;
    let msg_text = msg_text.to_lowercase();
    if msg_text.starts_with("1337") {
        return DAILY1337;
    }
    else if msg_text.starts_with("guess ") {
        return GUESS(msg_text.split_at(6).1.into());
    }
    NONE
}

pub fn handle_command(server: &IrcServer, msg: Message, msg_text: String) {
    use self::Command::*;
    use irc::proto::Command::PRIVMSG;
    let cmd = get_command(msg_text);
    //add whois for user
    match cmd {
        DAILY1337 => server.send(PRIVMSG(msg.response_target().unwrap().into(), "u l33t h4xx0r you :)".into())),
        GUESS(arg) => server.send(PRIVMSG(msg.response_target().unwrap().into(), arg)),
        NONE => server.send(PRIVMSG(msg.response_target().unwrap().into(), "none xd".into()))
    };
}

fn exec_command(account: String, arg: String) {
}

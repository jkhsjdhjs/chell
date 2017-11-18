extern crate base64;
extern crate futures;
extern crate regex;

use irc::client::prelude::*;
use irc::client::server::ServerStream;
use irc::error::Error;
use irc::error::ErrorKind::Msg;
use irc::proto::command::Command::*;
use irc::proto::CapSubCommand::*;
use irc::proto::response::Response::*;
use self::base64::encode;
use self::futures::future::ok;
use self::regex::Regex;

fn send_msg(server: &IrcServer, i: u8) -> Result<(), Error> {
    match i {
        0 => {
            server.send_cap_ls(NegotiationVersion::V302)?;
            server.send(NICK(server.config().nickname().to_owned()))?;
            server.send(USER(
                server.config().username().to_owned(),
                "0".to_owned(),
                server.config().real_name().to_owned(),
            ))
        }
        1 => server.send_cap_req(&[Capability::Sasl]),
        2 => server.send_sasl_plain(),
        3 => server.send_sasl(&encode(
            &format!(
                "{}\u{0000}{}\u{0000}{}",
                server.config().nickname(),
                server.config().username(),
                server.config().nick_password()
            )[..],
        )),
        4 => server.send(CAP(None, END, None, None)),
        _ => unreachable!(),
    }
}

fn wait_for_msg<S>(stream: S, i: u8) -> Result<(Option<Message>, impl Stream<Item = Message>), ()>
where
    S: Stream<Item = Message>,
{
    stream
        .skip_while(move |msg| match i {
            0 => match msg.command {
                CAP(_, LS, None, _) => ok(false),
                _ => ok(true),
            },
            1 => match msg.command {
                CAP(_, ACK, None, _) => ok(false),
                _ => ok(true),
            },
            2 => match msg.command {
                AUTHENTICATE(_) => ok(false),
                Response(RPL_SASLMECHS, _, _) => ok(false),
                _ => ok(true),
            },
            3 => match msg.command {
                Response(res, _, _) => match res {
                    RPL_SASLSUCCESS => ok(false),
                    ERR_NICKLOCKED => ok(false),
                    ERR_SASLFAIL => ok(false),
                    _ => ok(true),
                },
                _ => ok(true),
            },
            _ => unreachable!(),
        })
        .into_future()
        .wait()
        .map_err(|_| ())
}

fn check_msg(msg: Message, server: &IrcServer, i: u8) -> Result<(), Error> {
    match i {
        0 => match msg.command {
            CAP(_, _, _, args) => {
                if !Regex::new(r"(?:^|\s)sasl(?:$|\s)")
                    .unwrap()
                    .is_match(&args.unwrap_or("".into())[..])
                {
                    return Err(Error::from_kind(
                        Msg("Server doesn't support sasl".to_owned()),
                    ));
                }
                Ok(())
            }
            _ => unreachable!(),
        },
        1 => match msg.command {
            CAP(nick, _, _, args) => {
                if nick.unwrap_or("".into()) != server.config().nickname()
                    || args.unwrap_or("".into()) != "sasl"
                {
                    return Err(Error::from_kind(
                        Msg("Error while checking sasl ack message".to_owned()),
                    ));
                }
                Ok(())
            }
            _ => unreachable!(),
        },
        2 => match msg.command {
            AUTHENTICATE(arg) => {
                if arg != "+" {
                    return Err(Error::from_kind(
                        Msg("SASL Authentication Error".to_owned()),
                    ));
                }
                Ok(())
            }
            Response(RPL_SASLMECHS, args, _) => Err(Error::from_kind(Msg(
                [
                    "PLAIN SASL is not supported by this server! Following mechanisms are: ",
                    &format!("{:#?}", args)[..],
                ].join(""),
            ))),
            _ => unreachable!(),
        },
        3 => match msg.command {
            Response(res, _, _) => match res {
                RPL_SASLSUCCESS => Ok(()),
                ERR_NICKLOCKED => Err(Error::from_kind(Msg("Your account is locked!".to_owned()))),
                ERR_SASLFAIL => Err(Error::from_kind(
                    Msg("SASL Authentication failed!".to_owned()),
                )),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

//fn authenticate<'a, S>(server: &IrcServer, stream: S, i: u8) -> Result<S, ()>
//where S: Stream<Item = Message> + 'a
//      {
//    if i > 3 {
//        return Ok(stream);
//    }
//    send_msg(server, i);
//    let (msg, stream) = wait_for_msg(stream, i)?;
//    let msg = msg.ok_or(())?;
//    check_msg(msg, server, i);
//    let stream = authenticate(server, stream, i + 1)?;
//    Ok(stream)
//}

pub fn auth(server: &IrcServer, stream: ServerStream) -> Result<impl Stream<Item = Message>, ()> {
    //let mut stream_ref: S = &stream;

    //let stream = (0..3).fold(Ok(stream), |acc, i| {
    //    let acc = acc?;
    //    send_msg(server, i);
    //    let (msg, stream) = wait_for_msg(&acc, i)?;
    //    let msg = msg.ok_or(())?;
    //    check_msg(msg, server, i);
    //    Ok(stream)
    //})?;


    send_msg(server, 0);
    let (msg, stream) = wait_for_msg(stream, 0)?;
    let msg = msg.ok_or(())?;
    check_msg(msg, server, 0);

    send_msg(server, 1);
    let (msg, stream) = wait_for_msg(stream, 1)?;
    let msg = msg.ok_or(())?;
    check_msg(msg, server, 1);

    send_msg(server, 2);
    let (msg, stream) = wait_for_msg(stream, 2)?;
    let msg = msg.ok_or(())?;
    check_msg(msg, server, 2);

    send_msg(server, 3);
    let (msg, stream) = wait_for_msg(stream, 3)?;
    let msg = msg.ok_or(())?;
    check_msg(msg, server, 3);


    //let stream_result = authenticate(server, stream, 0);

    //for i in 0..3 {
    //    send_msg(server, i);
    //    let (msg, stream) = wait_for_msg(&stream_ref, i)?;
    //    let msg = msg.ok_or(())?;
    //    check_msg(msg, server, i);
    //    stream_ref = stream;
    //}

    //end authentication
    send_msg(server, 4);

    Ok(stream)
}

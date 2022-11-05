use common::SOCKET_PATH;
use std::{
    error::Error,
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    process::Command,
};
use x11rb::{
    self,
    properties::WmClass,
    protocol::xproto::{AtomEnum, ConnectionExt, GetPropertyReply},
    rust_connection::RustConnection,
};

fn handle_stream(mut unix_stream: UnixStream, conn: &RustConnection) {
    let mut buffer = [0; 1];

    unix_stream
        .read_exact(&mut buffer)
        .expect("Failed at reading the unix stream");

    let Some(focus) = get_focus(&conn)else{
        return
    };

    let received: String = buffer.into_iter().map(|c| c as char).collect();

    let bruh = |a: &str, x: &str| {
        println!("releasing {}, pressing {}", a, x);
        let a = match a {
            " " => "space",
            _ => a,
        };

        Command::new("xdotool")
            .arg("keyup")
            .arg("--delay")
            .arg("0")
            .arg("ctrl")
            .arg(a)
            .arg("key")
            .arg("--delay")
            .arg("2")
            .arg(format!("ctrl+{}", x))
            .arg("keydown")
            .arg("--delay")
            .arg("0")
            .arg("ctrl")
            .output()
            .expect("failed to execute xdotool");
    };

    if !matches!(focus, Focus::Other) {
        if let Some(mapped) = focus.remap(&received) {
            bruh(&received, mapped);
        } else {
            if received == "[" {
                bruh("bracketleft", "Escape");
            }
            {
                let output = Command::new("xdotool")
                    .arg("keyup")
                    .arg("--delay")
                    .arg("0")
                    .arg("ctrl")
                    .arg(&received)
                    .arg("key")
                    .arg("--delay")
                    .arg("0")
                    .arg(remap(&received))
                    .arg("keydown")
                    .arg("--delay")
                    .arg("0")
                    .arg("ctrl")
                    .output()
                    .expect("failed to execute xdotool");
            }
        }

        // std::io::stdout().write_all(&output.stdout).unwrap();
        // std::io::stderr().write_all(&output.stderr).unwrap();
    } else {
        bruh(&received, &received);
    }

    println!("{:?}", buffer);
}

macro_rules! mmm {
    ( $matchee: ident $( $char: literal $mapped: literal)* ) => {
        match $matchee {
          $($char => Some($mapped),)*
          _ => {
              None
          }
        }
    };
}

fn remap(c: &str) -> &'static str {
    match c {
        "h" => "BackSpace",
        "j" => "Enter",
        "i" => "Tab",
        "n" => "Down",
        "p" => "Up",
        "[" => "Escape",
        _ => panic!("unexpected key"),
    }
}

enum Focus {
    Discord,
    Chrome,
    Other,
}
impl Focus {
    fn remap(self, received: &str) -> Option<&'static str> {
        match self {
            Self::Chrome => mmm!(received "m" "j" " " "n"),
            _ => None,
        }
    }
}

fn get_focus(conn: &RustConnection) -> Option<Focus> {
    let f = conn.get_input_focus().expect("cannot get input focus");
    let f = f.reply().unwrap();

    let a = conn
        .get_property(
            false,
            f.focus,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            0,
            u32::MAX,
        )
        .unwrap();
    let a = a.reply().unwrap();

    let (instance, class) = parse_wm_class(&a)?;

    println!("Window instance: {:?}", instance);
    println!("Window class: {:?}", class);
    Some(match instance {
        "discord" => Focus::Discord,
        "google-chrome" => Focus::Chrome,
        _ => Focus::Other,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let (conn, _) = x11rb::connect(None)?;

    std::fs::remove_file(SOCKET_PATH).ok();
    let unix_listener = UnixListener::bind(SOCKET_PATH).expect("cannot create socket");

    loop {
        let (mut unix_stream, socket_address) = unix_listener
            .accept()
            .expect("Failed at accepting a connection on the unix listener");
        handle_stream(unix_stream, &conn);
    }

    Ok(())
}

fn parse_string_property(property: &GetPropertyReply) -> &str {
    std::str::from_utf8(&property.value).unwrap_or("Invalid utf8")
}

fn parse_wm_class(property: &GetPropertyReply) -> Option<(&str, &str)> {
    if property.format != 8 {
        // "Malformed property: wrong format",
        return None;
    }
    let value = &property.value;
    // The property should contain two null-terminated strings. Find them.
    if let Some(middle) = value.iter().position(|&b| b == 0) {
        let (instance, class) = value.split_at(middle);
        // Skip the null byte at the beginning
        let mut class = &class[1..];
        // Remove the last null byte from the class, if it is there.
        if class.last() == Some(&0) {
            class = &class[..class.len() - 1];
        }
        let instance = std::str::from_utf8(instance);
        let class = std::str::from_utf8(class);
        Some((
            instance.unwrap_or("Invalid utf8"),
            class.unwrap_or("Invalid utf8"),
        ))
    } else {
        // "Missing null byte"
        None
    }
}

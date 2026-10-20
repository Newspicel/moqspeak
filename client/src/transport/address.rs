//! Where to connect, and which transport takes us there. The scheme decides: `ts3server://` and
//! `ts3://` reach a TeamSpeak 3 server, anything else reaches a moqspeak Worker.

use anyhow::{Result, anyhow};

/// A connection target, already sorted to the transport that serves it.
#[derive(Clone, Debug, PartialEq)]
pub enum Address {
    Moq(Moq),
    Ts3(Ts3),
}

/// A moqspeak Worker and the server hosted on it, such as
/// `moqspeak.example.workers.dev/myserver`.
#[derive(Clone, Debug, PartialEq)]
pub struct Moq {
    pub ws_url: String,
    pub server: String,
}

/// A TeamSpeak 3 server. `addr` carries a port only where one was given, so the resolver may try
/// SRV and TSDNS before it falls back to port 9987.
#[derive(Clone, Debug, PartialEq)]
pub struct Ts3 {
    pub addr: String,
    pub host: String,
    pub password: Option<String>,
    pub channel: Option<String>,
    pub channel_password: Option<String>,
}

impl Address {
    pub fn parse(input: &str) -> Result<Self> {
        let input = input.trim();
        if input.is_empty() {
            return Err(anyhow!("enter a server address"));
        }
        for scheme in ["ts3server://", "ts3://"] {
            if let Some(rest) = input.strip_prefix(scheme) {
                return Ts3::parse(rest).map(Address::Ts3);
            }
        }
        Moq::parse(input).map(Address::Moq)
    }

    /// What to call this server while connecting to it.
    pub fn label(&self) -> &str {
        match self {
            Address::Moq(a) => &a.server,
            Address::Ts3(a) => &a.host,
        }
    }
}

impl Moq {
    fn parse(input: &str) -> Result<Self> {
        let (scheme, rest) = if let Some(r) = input.strip_prefix("wss://") {
            ("wss", r)
        } else if let Some(r) = input.strip_prefix("ws://") {
            ("ws", r)
        } else if let Some(r) = input.strip_prefix("https://") {
            ("wss", r)
        } else if let Some(r) = input.strip_prefix("http://") {
            ("ws", r)
        } else if input.starts_with("localhost") || input.starts_with("127.0.0.1") {
            ("ws", input)
        } else {
            ("wss", input)
        };
        let rest = rest.trim_end_matches('/');
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let server = path
            .trim_start_matches("s/")
            .split('/')
            .next_back()
            .unwrap_or("");
        let server = if server.is_empty() { "public" } else { server };
        if !server
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
        {
            return Err(anyhow!(
                "server names use letters, digits, '.', '-' and '_'"
            ));
        }
        Ok(Self {
            ws_url: format!("{scheme}://{host}/s/{server}"),
            server: server.to_owned(),
        })
    }
}

impl Ts3 {
    fn parse(input: &str) -> Result<Self> {
        let input = input.trim_end_matches('/');
        let (target, query) = input.split_once('?').unwrap_or((input, ""));
        let target = target.trim();
        if target.is_empty() {
            return Err(anyhow!("enter a TeamSpeak server address"));
        }
        let mut port = None;
        let mut password = None;
        let mut channel = None;
        let mut channel_password = None;
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let value = decode(value);
            match key {
                "port" => port = Some(value),
                "password" => password = Some(value),
                "channel" => channel = Some(value),
                "channelpassword" => channel_password = Some(value),
                _ => {}
            }
        }
        let host = host_of(target);
        let addr = match port {
            // A port in the address wins over one in the query.
            Some(p) if !has_port(target) => format!("{target}:{p}"),
            _ => target.to_owned(),
        };
        Ok(Self {
            addr,
            host: host.to_owned(),
            password: password.filter(|p| !p.is_empty()),
            channel: channel.filter(|c| !c.is_empty()),
            channel_password: channel_password.filter(|p| !p.is_empty()),
        })
    }
}

/// Whether `target` already names a port, allowing for a bracketed IPv6 address.
fn has_port(target: &str) -> bool {
    match target.rfind(']') {
        Some(bracket) => target[bracket..].contains(':'),
        None => target.matches(':').count() == 1,
    }
}

/// The host of `target`, without its port.
fn host_of(target: &str) -> &str {
    if let Some(bracket) = target.rfind(']') {
        return target[..=bracket]
            .trim_start_matches('[')
            .trim_end_matches(']');
    }
    if target.matches(':').count() == 1 {
        return target.split(':').next().unwrap_or(target);
    }
    target
}

/// Percent decoding, enough for the values a `ts3server://` link carries.
fn decode(value: &str) -> String {
    let bytes = value.replace('+', " ").into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moq(input: &str) -> Moq {
        match Address::parse(input).unwrap() {
            Address::Moq(a) => a,
            other => panic!("expected a Worker address, got {other:?}"),
        }
    }

    fn ts3(input: &str) -> Ts3 {
        match Address::parse(input).unwrap() {
            Address::Ts3(a) => a,
            other => panic!("expected a TeamSpeak address, got {other:?}"),
        }
    }

    #[test]
    fn parses_worker_addresses() {
        let a = moq("moqspeak.example.workers.dev/demo");
        assert_eq!(a.ws_url, "wss://moqspeak.example.workers.dev/s/demo");
        let a = moq("https://x.dev/s/team/");
        assert_eq!(a.ws_url, "wss://x.dev/s/team");
        let a = moq("localhost:8787");
        assert_eq!(a.ws_url, "ws://localhost:8787/s/public");
        assert!(Address::parse("x.dev/bad name").is_err());
    }

    #[test]
    fn a_teamspeak_scheme_picks_the_teamspeak_transport() {
        let a = ts3("ts3server://voice.example.com");
        assert_eq!(a.addr, "voice.example.com");
        assert_eq!(a.host, "voice.example.com");
        assert_eq!(a.password, None);
        let a = ts3("ts3://voice.example.com:9988");
        assert_eq!(a.addr, "voice.example.com:9988");
        assert_eq!(a.host, "voice.example.com");
    }

    #[test]
    fn reads_the_query_a_teamspeak_link_carries() {
        let a = ts3("ts3server://voice.example.com?port=9988&password=hunter2&channel=Lobby%2FAFK");
        assert_eq!(a.addr, "voice.example.com:9988");
        assert_eq!(a.password.as_deref(), Some("hunter2"));
        assert_eq!(a.channel.as_deref(), Some("Lobby/AFK"));
    }

    #[test]
    fn a_port_in_the_address_wins_over_the_query() {
        let a = ts3("ts3server://voice.example.com:10011?port=9987");
        assert_eq!(a.addr, "voice.example.com:10011");
    }

    #[test]
    fn keeps_a_bracketed_address_whole() {
        let a = ts3("ts3server://[2001:db8::1]:9987");
        assert_eq!(a.addr, "[2001:db8::1]:9987");
        assert_eq!(a.host, "2001:db8::1");
        let a = ts3("ts3server://[2001:db8::1]?port=9987");
        assert_eq!(a.addr, "[2001:db8::1]:9987");
    }

    #[test]
    fn labels_a_server_by_what_the_person_reaches() {
        assert_eq!(Address::parse("x.dev/team").unwrap().label(), "team");
        assert_eq!(
            Address::parse("ts3server://voice.example.com")
                .unwrap()
                .label(),
            "voice.example.com"
        );
    }
}

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::types::RawEvent;
use crate::{ConfigSnapshot, Endpoint, Error, Event, Result, Status, VersionInfo};

/// How long a request waits for its response by default.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// A connection to the driver's control API.
///
/// Requests are sequential: each call writes one line and reads lines until
/// its response.
pub struct Client {
    reader: Box<dyn BufRead + Send>,
    writer: Box<dyn Write + Send>,
    next_id: u64,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("next_id", &self.next_id)
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Connects to `endpoint`, with [`DEFAULT_TIMEOUT`] on each request.
    pub fn connect(endpoint: &Endpoint) -> Result<Self> {
        Self::connect_with_timeout(endpoint, Some(DEFAULT_TIMEOUT))
    }

    /// Connects to `endpoint`; `timeout` bounds each read and write (`None`
    /// waits forever, as a subscription does). Named pipes on Windows have
    /// no timeout.
    pub fn connect_with_timeout(endpoint: &Endpoint, timeout: Option<Duration>) -> Result<Self> {
        let (reader, writer) = transport::connect(endpoint, timeout)?;
        Ok(Self::from_streams(reader, writer))
    }

    /// A client over any pair of streams (tests, other transports).
    pub fn from_streams<R, W>(reader: R, writer: W) -> Self
    where
        R: Read + Send + 'static,
        W: Write + Send + 'static,
    {
        Self {
            reader: Box::new(BufReader::new(reader)),
            writer: Box::new(writer),
            next_id: 1,
        }
    }

    /// Calls `method` and decodes its result.
    pub fn call<T: DeserializeOwned>(&mut self, method: &str, params: &Value) -> Result<T> {
        let id = self.send(method, params)?;
        loop {
            let message = read_message(&mut self.reader)?;
            match message.get("id") {
                // An event on a request connection: not ours, skip it.
                None | Some(Value::Null) if message.contains_key("event") => {}
                Some(value) if value.as_u64() == Some(id) => return decode_response(message),
                other => {
                    return Err(Error::Protocol(format!(
                        "expected the response to request {id}, got id {}",
                        other.map_or_else(|| "none".to_owned(), Value::to_string)
                    )));
                }
            }
        }
    }

    /// `version`: the driver's and the API's versions.
    pub fn version(&mut self) -> Result<VersionInfo> {
        self.call("version", &json!({}))
    }

    /// `status`: the guns and their state.
    pub fn status(&mut self) -> Result<Status> {
        self.call("status", &json!({}))
    }

    /// `get_config`: the configuration file's path and every key.
    pub fn get_config(&mut self) -> Result<ConfigSnapshot> {
        self.call("get_config", &json!({}))
    }

    /// `set_config`: writes `values` into the configuration file (the rest
    /// of the file kept) and restarts the guns; returns the new snapshot.
    pub fn set_config(&mut self, values: &BTreeMap<String, String>) -> Result<ConfigSnapshot> {
        self.call("set_config", &json!({ "values": values }))
    }

    /// `subscribe`: turns this connection into a stream of events.
    ///
    /// The driver may acknowledge the request with a response (`{"id": n,
    /// "result": ...}`) before the events, or send events straight away;
    /// both are accepted. An error response is returned as an error.
    pub fn subscribe(mut self) -> Result<Subscription> {
        let id = self.send("subscribe", &json!({}))?;
        let mut pending = VecDeque::new();
        let message = read_message(&mut self.reader)?;
        match message.get("id") {
            Some(value) if value.as_u64() == Some(id) => {
                decode_response::<Value>(message)?;
            }
            None | Some(Value::Null) => pending.push_back(decode_event(message)?),
            Some(other) => {
                return Err(Error::Protocol(format!(
                    "expected the response to subscribe ({id}), got id {other}"
                )));
            }
        }
        Ok(Subscription {
            reader: self.reader,
            pending,
            _writer: self.writer,
        })
    }

    fn send(&mut self, method: &str, params: &Value) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;
        let mut line =
            serde_json::to_vec(&json!({ "id": id, "method": method, "params": params }))?;
        line.push(b'\n');
        self.writer.write_all(&line)?;
        self.writer.flush()?;
        Ok(id)
    }
}

/// The events of a `subscribe` connection; ends when the driver closes it.
pub struct Subscription {
    reader: Box<dyn BufRead + Send>,
    pending: VecDeque<Event>,
    // Kept so that the connection stays open in both directions.
    _writer: Box<dyn Write + Send>,
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Subscription")
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl Subscription {
    /// Waits for the next event.
    pub fn next_event(&mut self) -> Result<Event> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        loop {
            let message = read_message(&mut self.reader)?;
            if message.contains_key("event") {
                return decode_event(message);
            }
            // A response line (`id`) on a subscription carries no event.
        }
    }
}

impl Iterator for Subscription {
    type Item = Result<Event>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_event() {
            Err(Error::Closed) => None,
            other => Some(other),
        }
    }
}

/// Reads the next non-empty line as a JSON object.
fn read_message(reader: &mut dyn BufRead) -> Result<Map<String, Value>> {
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Err(Error::Closed);
        }
        if line.trim().is_empty() {
            continue;
        }
        return match serde_json::from_str(&line)? {
            Value::Object(map) => Ok(map),
            other => Err(Error::Protocol(format!(
                "expected a JSON object, got {other}"
            ))),
        };
    }
}

fn decode_response<T: DeserializeOwned>(mut message: Map<String, Value>) -> Result<T> {
    match message.remove("error") {
        None | Some(Value::Null) => {}
        Some(Value::String(error)) => return Err(Error::Service(error)),
        Some(other) => return Err(Error::Service(other.to_string())),
    }
    let result = message.remove("result").unwrap_or(Value::Null);
    Ok(serde_json::from_value(result)?)
}

fn decode_event(message: Map<String, Value>) -> Result<Event> {
    let raw: RawEvent = serde_json::from_value(Value::Object(message))?;
    raw.try_into()
}

#[cfg(unix)]
mod transport {
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use crate::{Endpoint, Result};

    pub fn connect(
        endpoint: &Endpoint,
        timeout: Option<Duration>,
    ) -> Result<(UnixStream, UnixStream)> {
        let stream = UnixStream::connect(endpoint.path())?;
        stream.set_read_timeout(timeout)?;
        stream.set_write_timeout(timeout)?;
        let reader = stream.try_clone()?;
        Ok((reader, stream))
    }
}

#[cfg(windows)]
mod transport {
    use std::fs::{File, OpenOptions};
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::{Endpoint, Result};

    /// `ERROR_PIPE_BUSY`: every instance of the pipe is in use.
    const ERROR_PIPE_BUSY: i32 = 231;
    const BUSY_WAIT: Duration = Duration::from_secs(2);

    pub fn connect(endpoint: &Endpoint, _timeout: Option<Duration>) -> Result<(File, File)> {
        let start = Instant::now();
        let pipe = loop {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .open(endpoint.path())
            {
                Ok(pipe) => break pipe,
                Err(err)
                    if err.raw_os_error() == Some(ERROR_PIPE_BUSY)
                        && start.elapsed() < BUSY_WAIT =>
                {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(err) => return Err(err.into()),
            }
        };
        let reader = pipe.try_clone()?;
        Ok((reader, pipe))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::GunState;

    /// A writer whose bytes the test can read back.
    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Sink {
        fn lines(&self) -> Vec<Value> {
            let bytes = self.0.lock().unwrap();
            std::str::from_utf8(&bytes)
                .unwrap()
                .lines()
                .map(|l| serde_json::from_str(l).unwrap())
                .collect()
        }
    }

    fn client(responses: &str) -> (Client, Sink) {
        let sink = Sink::default();
        (
            Client::from_streams(Cursor::new(responses.to_owned().into_bytes()), sink.clone()),
            sink,
        )
    }

    #[test]
    fn request_shape_and_ids() {
        let (mut c, sink) = client(
            "{\"id\":1,\"result\":{\"driver\":\"0.2.0\",\"api\":1}}\n{\"id\":2,\"result\":{\"guns\":[]}}\n",
        );
        assert_eq!(
            c.version().unwrap(),
            VersionInfo {
                driver: "0.2.0".into(),
                api: 1
            }
        );
        assert_eq!(c.status().unwrap().guns, []);
        assert_eq!(
            sink.lines(),
            vec![
                json!({"id": 1, "method": "version", "params": {}}),
                json!({"id": 2, "method": "status", "params": {}}),
            ]
        );
    }

    #[test]
    fn set_config_sends_values() {
        let (mut c, sink) =
            client("{\"id\":1,\"result\":{\"path\":\"/etc/x\",\"values\":{\"Joystick\":\"1\"}}}\n");
        let values = BTreeMap::from([("Joystick".to_owned(), "1".to_owned())]);
        let snapshot = c.set_config(&values).unwrap();
        assert_eq!(snapshot.values["Joystick"], "1");
        assert_eq!(
            sink.lines()[0]["params"],
            json!({"values": {"Joystick": "1"}})
        );
    }

    #[test]
    fn error_response() {
        let (mut c, _) = client("{\"id\":1,\"error\":\"unknown key Foo\"}\n");
        match c.get_config() {
            Err(Error::Service(message)) => assert_eq!(message, "unknown key Foo"),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn mismatched_id_is_a_protocol_error() {
        let (mut c, _) = client("{\"id\":7,\"result\":null}\n");
        assert!(matches!(c.status(), Err(Error::Protocol(_))));
    }

    #[test]
    fn events_and_blank_lines_before_a_response_are_skipped() {
        let (mut c, _) = client(
            "\n{\"event\":\"state\",\"gun\":{\"player\":1,\"state\":\"aiming\"}}\n{\"id\":1,\"result\":{\"guns\":[]}}\n",
        );
        assert_eq!(c.status().unwrap().guns, []);
    }

    #[test]
    fn closed_connection() {
        let (mut c, _) = client("");
        let err = c.status().unwrap_err();
        assert!(matches!(err, Error::Closed));
        assert!(err.is_unreachable());
    }

    #[test]
    fn invalid_json() {
        let (mut c, _) = client("not json\n");
        assert!(matches!(c.status(), Err(Error::Json(_))));
    }

    #[test]
    fn subscribe_with_acknowledgement() {
        let (c, _) = client(concat!(
            "{\"id\":1,\"result\":null}\n",
            "{\"event\":\"gun_added\",\"gun\":{\"player\":1,\"state\":\"aiming\"}}\n",
            "{\"event\":\"state\",\"gun\":{\"player\":1,\"state\":\"aimed_away\"}}\n",
        ));
        let events: Vec<Event> = c.subscribe().unwrap().map(Result::unwrap).collect();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], Event::GunAdded(g) if g.player == 1));
        assert!(matches!(&events[1], Event::State(g) if g.state == GunState::AimedAway));
    }

    #[test]
    fn subscribe_without_acknowledgement() {
        let (c, _) =
            client("{\"event\":\"gun_removed\",\"gun\":{\"player\":2,\"state\":\"stopped\"}}\n");
        let events: Vec<Event> = c.subscribe().unwrap().map(Result::unwrap).collect();
        assert!(matches!(&events[..], [Event::GunRemoved(g)] if g.player == 2));
    }

    #[test]
    fn subscribe_refused() {
        let (c, _) = client("{\"id\":1,\"error\":\"too many subscribers\"}\n");
        assert!(matches!(c.subscribe(), Err(Error::Service(_))));
    }
}

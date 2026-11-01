use std::fs::File;
use std::io::{self, IoSlice, Read, Write};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

pub const HEADER: usize = 9;
const RELAY_BUFFER: usize = 64 * 1024;
const CUT_HEADER_AT: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Out,
    Err,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Record {
    Begin(u64),
    Finish(u64),
    AfterInputs,
    Unpack,
    Emit,
    Stdout(u64),
    Stderr(u64),
}

impl Record {
    pub fn header(self) -> [u8; HEADER] {
        let (tag, operand) = match self {
            Record::Begin(index) => (b'B', index),
            Record::Finish(index) => (b'F', index),
            Record::AfterInputs => (b'A', 0),
            Record::Unpack => (b'U', 0),
            Record::Emit => (b'W', 0),
            Record::Stdout(len) => (b'O', len),
            Record::Stderr(len) => (b'E', len),
        };
        let mut bytes = [0; HEADER];
        bytes[0] = tag;
        bytes[1..].copy_from_slice(&operand.to_le_bytes());
        bytes
    }

    pub fn parse(bytes: &[u8; HEADER]) -> Option<Record> {
        let mut operand = [0; 8];
        operand.copy_from_slice(&bytes[1..]);
        let operand = u64::from_le_bytes(operand);
        Some(match bytes[0] {
            b'B' => Record::Begin(operand),
            b'F' => Record::Finish(operand),
            b'A' => Record::AfterInputs,
            b'U' => Record::Unpack,
            b'W' => Record::Emit,
            b'O' => Record::Stdout(operand),
            b'E' => Record::Stderr(operand),
            _ => return None,
        })
    }
}

pub fn write_record(out: &mut impl Write, record: Record, body: &[u8]) -> io::Result<()> {
    let header = record.header();
    let (mut head, mut rest) = (&header[..], body);
    while !head.is_empty() || !rest.is_empty() {
        let written = match out.write_vectored(&[IoSlice::new(head), IoSlice::new(rest)]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => written,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let from_head = written.min(head.len());
        head = &head[from_head..];
        rest = rest.get(written - from_head..).unwrap_or_default();
    }
    Ok(())
}

static CHANNEL: OnceLock<Mutex<File>> = OnceLock::new();

#[cfg(unix)]
fn duplicate_stdout() -> io::Result<File> {
    use std::os::fd::AsFd;
    Ok(File::from(io::stdout().as_fd().try_clone_to_owned()?))
}

#[cfg(not(unix))]
fn duplicate_stdout() -> io::Result<File> {
    Err(io::Error::other("the worker's channel needs a Unix target"))
}

pub fn open_channel() -> io::Result<()> {
    let file = duplicate_stdout()?;
    CHANNEL
        .set(Mutex::new(file))
        .map_err(|_| io::Error::other("the worker's channel is already open"))
}

fn channel() -> io::Result<MutexGuard<'static, File>> {
    let channel = CHANNEL
        .get()
        .ok_or_else(|| io::Error::other("the worker's channel is not open"))?;
    Ok(channel.lock().unwrap_or_else(PoisonError::into_inner))
}

fn send(record: Record, body: &[u8]) -> io::Result<()> {
    write_record(&mut *channel()?, record, body)
}

pub fn write_marker(record: Record) -> io::Result<()> {
    send(record, &[])
}

pub fn cut_record(record: Record, body: &[u8]) -> ! {
    let header = record.header();
    let held = channel();
    if let Ok(mut out) = held {
        let _ = if body.is_empty() {
            out.write_all(&header[..CUT_HEADER_AT])
        } else {
            out.write_all(&header)
                .and_then(|()| out.write_all(&body[..body.len() / 2]))
        };
        std::process::abort()
    }
    std::process::abort()
}

pub struct FramedWriter {
    stream: Stream,
    cut: bool,
}

impl FramedWriter {
    pub fn new(stream: Stream) -> Self {
        Self { stream, cut: false }
    }

    pub fn cut_midway(self, cut: bool) -> Self {
        Self { cut, ..self }
    }
}

impl Write for FramedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let len = u64::try_from(buf.len()).map_err(io::Error::other)?;
        let record = match self.stream {
            Stream::Out => Record::Stdout(len),
            Stream::Err => Record::Stderr(len),
        };
        if self.cut {
            cut_record(record, buf);
        }
        send(record, buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Event<'a> {
    Record(Record),
    Body(Stream, &'a [u8]),
}

#[derive(Debug)]
pub enum RelayEnd {
    Eof,
    Truncated,
    Sink(io::Error),
    Malformed,
    ReadFailed(io::Error),
}

impl RelayEnd {
    pub fn kills_the_worker(&self) -> bool {
        matches!(
            self,
            RelayEnd::Sink(_) | RelayEnd::Malformed | RelayEnd::ReadFailed(_)
        )
    }
}

#[derive(Default)]
struct Decoder {
    header: [u8; HEADER],
    have: usize,
    body: Option<(Stream, u64)>,
}

impl Decoder {
    fn feed<'a>(
        &mut self,
        mut input: &'a [u8],
        sink: &mut impl FnMut(Event<'a>) -> io::Result<()>,
    ) -> Result<(), RelayEnd> {
        while !input.is_empty() {
            input = match self.body {
                Some(body) => self.body_bytes(body, input, sink)?,
                None => self.header_bytes(input, sink)?,
            };
        }
        Ok(())
    }

    fn body_bytes<'a>(
        &mut self,
        (stream, left): (Stream, u64),
        input: &'a [u8],
        sink: &mut impl FnMut(Event<'a>) -> io::Result<()>,
    ) -> Result<&'a [u8], RelayEnd> {
        let take = usize::try_from(left).map_or(input.len(), |left| left.min(input.len()));
        let (chunk, rest) = input.split_at(take);
        sink(Event::Body(stream, chunk)).map_err(RelayEnd::Sink)?;
        let left = left - take as u64;
        self.body = (left > 0).then_some((stream, left));
        Ok(rest)
    }

    fn header_bytes<'a>(
        &mut self,
        input: &'a [u8],
        sink: &mut impl FnMut(Event<'a>) -> io::Result<()>,
    ) -> Result<&'a [u8], RelayEnd> {
        let take = (HEADER - self.have).min(input.len());
        self.header[self.have..self.have + take].copy_from_slice(&input[..take]);
        self.have += take;
        if self.have < HEADER {
            return Ok(&input[take..]);
        }
        self.have = 0;
        let record = Record::parse(&self.header).ok_or(RelayEnd::Malformed)?;
        match record {
            Record::Stdout(len) if len > 0 => self.body = Some((Stream::Out, len)),
            Record::Stderr(len) if len > 0 => self.body = Some((Stream::Err, len)),
            Record::Stdout(_) | Record::Stderr(_) => {}
            marker => sink(Event::Record(marker)).map_err(RelayEnd::Sink)?,
        }
        Ok(&input[take..])
    }

    fn mid_record(&self) -> bool {
        self.have > 0 || self.body.is_some()
    }
}

pub struct Relay<R> {
    reader: R,
    buffer: Vec<u8>,
    decoder: Decoder,
}

impl<R: Read> Relay<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: vec![0; RELAY_BUFFER],
            decoder: Decoder::default(),
        }
    }

    pub fn run(&mut self, mut sink: impl FnMut(Event<'_>) -> io::Result<()>) -> RelayEnd {
        loop {
            let read = match self.reader.read(&mut self.buffer) {
                Ok(0) => return self.at_end(),
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return RelayEnd::ReadFailed(error),
            };
            if let Err(end) = self.decoder.feed(&self.buffer[..read], &mut sink) {
                return end;
            }
        }
    }

    fn at_end(&self) -> RelayEnd {
        if self.decoder.mid_record() {
            return RelayEnd::Truncated;
        }
        RelayEnd::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::{write_record, Event, Record, Relay, RelayEnd, Stream, HEADER};
    use std::io::{self, IoSlice, Read, Write};

    const RECORDS: [Record; 7] = [
        Record::Begin(7),
        Record::Finish(u64::MAX),
        Record::AfterInputs,
        Record::Unpack,
        Record::Emit,
        Record::Stdout(3),
        Record::Stderr(0),
    ];

    #[test]
    fn every_record_round_trips_through_its_header() {
        for record in RECORDS {
            assert_eq!(Record::parse(&record.header()), Some(record));
        }
        let mut unknown = Record::Emit.header();
        unknown[0] = b'Z';
        assert_eq!(Record::parse(&unknown), None);
    }

    struct Trickle<'a> {
        bytes: &'a [u8],
        step: usize,
    }

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let take = self.step.min(self.bytes.len()).min(buf.len());
            buf[..take].copy_from_slice(&self.bytes[..take]);
            self.bytes = &self.bytes[take..];
            Ok(take)
        }
    }

    fn stream_of(frames: &[(Record, &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (record, body) in frames {
            write_record(&mut bytes, *record, body).unwrap();
        }
        bytes
    }

    type Bodies = Vec<(Stream, Vec<u8>)>;

    fn decode(bytes: &[u8], step: usize) -> (Vec<Record>, Bodies, RelayEnd) {
        let mut records = Vec::new();
        let mut bodies: Bodies = Vec::new();
        let end = Relay::new(Trickle { bytes, step }).run(|event| {
            match event {
                Event::Record(record) => records.push(record),
                Event::Body(stream, chunk) => match bodies.last_mut() {
                    Some((last, body)) if *last == stream => body.extend_from_slice(chunk),
                    _ => bodies.push((stream, chunk.to_vec())),
                },
            }
            Ok(())
        });
        (records, bodies, end)
    }

    #[test]
    fn frames_split_across_reads_decode_to_the_same_events() {
        let bytes = stream_of(&[
            (Record::Begin(0), b""),
            (Record::Stderr(5), b"warn\n"),
            (Record::Finish(0), b""),
            (Record::Emit, b""),
            (Record::Stdout(11), b"var a = 1;\n"),
            (Record::Stderr(0), b""),
            (Record::Stderr(4), b"done"),
        ]);
        for step in [1, 2, 8, 9, 10, 64 * 1024] {
            let (records, bodies, end) = decode(&bytes, step);
            assert!(matches!(end, RelayEnd::Eof), "{step}");
            assert_eq!(
                records,
                [Record::Begin(0), Record::Finish(0), Record::Emit],
                "{step}"
            );
            assert_eq!(
                bodies,
                [
                    (Stream::Err, b"warn\n".to_vec()),
                    (Stream::Out, b"var a = 1;\n".to_vec()),
                    (Stream::Err, b"done".to_vec())
                ],
                "{step}"
            );
        }
    }

    #[test]
    fn a_stream_cut_inside_a_record_is_truncated() {
        let bytes = stream_of(&[(Record::Begin(1), b""), (Record::Stdout(10), b"0123456789")]);
        for cut in [HEADER + 2, HEADER + HEADER + 4] {
            let (_, _, end) = decode(&bytes[..cut], 3);
            assert!(matches!(end, RelayEnd::Truncated), "{cut}");
        }
    }

    #[test]
    fn an_unknown_tag_is_malformed() {
        let (_, _, end) = decode(b"Z00000000", 9);
        assert!(matches!(end, RelayEnd::Malformed));
        assert!(end.kills_the_worker());
        assert!(!RelayEnd::Truncated.kills_the_worker());
    }

    struct Counter {
        writes: Vec<Vec<u8>>,
    }

    impl Write for Counter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.push(buf.to_vec());
            Ok(buf.len())
        }

        fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
            let joined: Vec<u8> = bufs.iter().flat_map(|buf| buf.iter().copied()).collect();
            self.write(&joined)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_record_goes_out_in_one_write_header_and_body_together() {
        let frames: [(Record, &[u8]); 3] = [
            (Record::Finish(10), b""),
            (Record::Begin(266), b""),
            (Record::Stdout(10), b"a\nb\nc\nd\ne\n"),
        ];
        for (record, body) in frames {
            let mut counter = Counter { writes: Vec::new() };
            write_record(&mut counter, record, body).unwrap();
            let mut whole = record.header().to_vec();
            whole.extend_from_slice(body);
            assert_eq!(counter.writes, [whole], "{record:?}");
        }
    }

    struct Partial {
        written: Vec<u8>,
    }

    impl Write for Partial {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let take = buf.len().min(4);
            self.written.extend_from_slice(&buf[..take]);
            Ok(take)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_partial_write_is_continued_to_the_end_of_the_body() {
        let mut partial = Partial {
            written: Vec::new(),
        };
        write_record(&mut partial, Record::Stderr(6), b"abcdef").unwrap();
        assert_eq!(
            partial.written,
            stream_of(&[(Record::Stderr(6), b"abcdef")])
        );
    }

    struct Failing<'a> {
        errors: Vec<io::ErrorKind>,
        bytes: &'a [u8],
    }

    impl Read for Failing<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if let Some(kind) = self.errors.pop() {
                return Err(kind.into());
            }
            let take = self.bytes.len().min(buf.len());
            buf[..take].copy_from_slice(&self.bytes[..take]);
            self.bytes = &self.bytes[take..];
            Ok(take)
        }
    }

    #[test]
    fn an_interrupted_read_is_retried_and_any_other_read_error_ends_the_relay() {
        let bytes = stream_of(&[(Record::Begin(4), b"")]);
        let mut records = Vec::new();
        let reader = Failing {
            errors: vec![io::ErrorKind::Interrupted],
            bytes: &bytes,
        };
        let end = Relay::new(reader).run(|event| {
            records.push(event == Event::Record(Record::Begin(4)));
            Ok(())
        });
        assert!(matches!(end, RelayEnd::Eof));
        assert_eq!(records, [true]);
        let reader = Failing {
            errors: vec![io::ErrorKind::Other],
            bytes: &bytes,
        };
        let end = Relay::new(reader).run(|_| Ok(()));
        assert!(matches!(end, RelayEnd::ReadFailed(_)));
        assert!(end.kills_the_worker());
    }

    #[test]
    fn a_failing_sink_ends_the_relay() {
        let bytes = stream_of(&[(Record::Stdout(2), b"ab")]);
        let end = Relay::new(&bytes[..]).run(|_| Err(io::Error::other("closed")));
        assert!(matches!(end, RelayEnd::Sink(_)));
    }
}

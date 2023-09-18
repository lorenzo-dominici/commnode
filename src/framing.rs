use super::Event;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_serde::formats::SymmetricalJson;

pub type FramedStream<T> = tokio_serde::Framed<Framed<T,  LengthDelimitedCodec>, Event, Event, SymmetricalJson<Event>>;

pub fn wrap_stream<T: AsyncRead + AsyncWrite>(stream: T) -> FramedStream<T> {
    let wrapped: Framed<T,  LengthDelimitedCodec> = Framed::new(stream, LengthDelimitedCodec::new());
    let framed: FramedStream<T> = FramedStream::new(wrapped, SymmetricalJson::default());
    framed
}
use super::Event;

use tokio::{io::{AsyncRead, AsyncWrite}, net::TcpStream};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_serde::{SymmetricallyFramed, formats::SymmetricalBincode};

pub type FramedStream<T> = SymmetricallyFramed<Framed<T, LengthDelimitedCodec>, Event, SymmetricalBincode<Event>>;

pub fn frame_stream<T: AsyncRead + AsyncWrite>(stream: T) -> FramedStream<T> {
    let codec = Framed::new(stream, LengthDelimitedCodec::new());
    let framed = FramedStream::new(codec, SymmetricalBincode::<Event>::default());
    framed
}
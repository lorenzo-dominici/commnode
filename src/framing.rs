//! This module offers functions and types for handling the the framing of streams.

use super::Event;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_serde::{SymmetricallyFramed, formats::SymmetricalBincode};

/// Alias for nested framed types.
pub type FramedStream<T> = SymmetricallyFramed<Framed<T, LengthDelimitedCodec>, Event, SymmetricalBincode<Event>>;

/// Returns the framed version of the input stream, with `LenghtDelimitedCodec` and `SymmetricalBincode` serialization.
pub fn frame_stream<T: AsyncRead + AsyncWrite>(stream: T) -> FramedStream<T> {
    let inner = Framed::new(stream, LengthDelimitedCodec::new());
    let framed = FramedStream::new(inner, SymmetricalBincode::<Event>::default());
    framed
}
//! This module offers functions and types for handling the the framing of streams.

use super::Event;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tokio_serde::{SymmetricallyFramed, formats::SymmetricalBincode};

/// Alias for nested framed types.
pub type FramedStream<T> = SymmetricallyFramed<Framed<T, LengthDelimitedCodec>, Event, SymmetricalBincode<Event>>;

/// Returns the framed version of the input stream, with `LenghtDelimitedCodec` and `SymmetricalBincode` serialization.
pub fn frame_stream<T: AsyncRead + AsyncWrite>(stream: T) -> FramedStream<T> {
    let inner = Framed::new(stream, LengthDelimitedCodec::builder().little_endian().length_field_length(4).new_codec());
    let framed = FramedStream::new(inner, SymmetricalBincode::<Event>::default());
    framed
}

/// Alias for nested framed types.
pub type FramedString<T> = Framed<T, LengthDelimitedCodec>;

/// Returns the framed version of the input stream, with `LenghtDelimitedCodec` and `SymmetricalBincode` serialization.
pub fn frame_string<T: AsyncRead + AsyncWrite>(stream: T) -> FramedString<T> {
    let framed = Framed::new(stream, LengthDelimitedCodec::builder().little_endian().length_field_length(4).new_codec());
    framed
}
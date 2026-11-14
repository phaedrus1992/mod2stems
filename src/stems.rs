//! Per-channel isolation renders.
//!
//! One render exports one tracker channel, with every other channel silenced.
//! Two guards prove that the silence did not change the song: the silenced
//! channels read back as silenced, and the render visits the same order and row
//! sequence as the unmuted render. Design: `docs/design.md` §4.1, §6.
//!
//! One export shares one [`PlaybackPolicy`]: the subsong, the repeat count, and
//! the duration limit. One export also shares one [`Timeline`]: the common end
//! is the longest completed render, and a shorter render is zero-padded at the
//! tail. Mono is the mean of the rendered pair, and it is lossy.
//! Design: `docs/design.md` §4.2, §4.3.
//!
//! The isolation core renders one channel at a time. Bounded worker
//! concurrency, temporary PCM files, and pair discovery are later work (#6).

pub mod error;
pub mod isolation;
pub mod layout;
pub mod policy;
pub mod position;
pub mod render;
pub mod silencing;
pub mod timeline;
pub mod trace;

pub use error::StemError;
pub use isolation::Reference;
pub use isolation::export_channel;
pub use isolation::reference_render;
pub use layout::ChannelLayout;
pub use layout::downmix;
pub use policy::PlaybackPolicy;
pub use position::Position;
pub use render::GuardedRender;
pub use render::read_guarded;
pub use silencing::Silencing;
pub use silencing::silencing_for;
pub use timeline::Timeline;
pub use trace::RowTrace;

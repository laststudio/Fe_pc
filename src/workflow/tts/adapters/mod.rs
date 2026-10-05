pub mod cloudflare;
pub mod custom_http;
pub mod edge;
pub mod fish_audio;
pub mod mimo;
pub mod minimax;
pub mod siliconflow;

pub use cloudflare::CloudflareAdapter;
pub use custom_http::CustomHttpAdapter;
pub use edge::{EdgeAdapter, EDGE_PRESET_VOICES, ensure_crypto_provider};
pub use fish_audio::FishAudioAdapter;
pub use mimo::MimoAdapter;
pub use minimax::{MiniMaxAdapter, MINIMAX_PRESET_VOICES};
pub use siliconflow::{SiliconFlowAdapter, SILICONFLOW_PRESET_MODELS};

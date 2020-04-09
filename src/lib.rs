mod fsm;
mod types;

pub type AsyncError = Box<dyn std::error::Error + Send + Sync>;

pub use fsm::Fsm;

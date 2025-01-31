use std::fmt::Debug;

use crate::error::ClientError;

pub mod opus;

pub trait AudioCodec: Send + Sync + Debug {
    fn new(sample_rate: u32, channels: usize) -> Result<Self, ClientError>
    where
        Self: Sized;
    fn encode(&self, data: Vec<f32>) -> Result<Vec<u8>, ClientError>;
    fn decode(&self, data: Vec<u8>) -> Result<Vec<f32>, ClientError>;
}

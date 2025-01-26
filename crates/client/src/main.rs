use audio::{codec::opus::OpusAudioCodec, cpal::CpalAudioHandler};
use client::{tokio::TokioClient, Client};
use error::ClientError;

mod audio;
mod client;
mod error;

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<(), ClientError> {
    let codec = CpalAudioHandler::<OpusAudioCodec>::new()?;
    let client = TokioClient::connect("127.0.0.1:1024".into(), codec).await?;
    client.run().await
}

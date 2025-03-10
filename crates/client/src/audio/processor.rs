use super::{codec::AudioCodec, SoundProcessor};
use crate::error::ClientError;
use common::packet::{AudioPacket, Packet};
use std::sync::Arc;
use std::u32;
use tokio::{
    select,
    sync::{
        mpsc::{self},
        oneshot, Mutex,
    },
};

pub struct AudioProcessor<Codec: AudioCodec> {
    codec: Arc<Mutex<Codec>>,

    stop_tx: Option<oneshot::Sender<()>>,
}

impl<Codec: AudioCodec> AudioProcessor<Codec> {}

#[async_trait::async_trait]
impl<Codec: AudioCodec + 'static> SoundProcessor for AudioProcessor<Codec> {
    type Codec = Codec;

    fn new() -> Result<Self, ClientError> {
        Ok(AudioProcessor {
            codec: Arc::new(Mutex::new(Codec::new()?)),
            stop_tx: None,
        })
    }

    async fn start(
        &mut self,
        mut mic_rx: mpsc::Receiver<Vec<f32>>,
        packet_sender: mpsc::Sender<Packet>,
    ) -> Result<(), ClientError> {
        println!("Starting audio handler");
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        {
            self.stop_tx = Some(stop_tx);
        }

        let codec = self.codec.clone();
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(20);
        let microphone_handle = tokio::spawn(async move {
            while let Some(audio_samples) = mic_rx.recv().await {
                match codec.lock().await.encode(audio_samples) {
                    Ok(encoded_data) => {
                        let _ = audio_tx.send(encoded_data).await;
                    }
                    Err(e) => {
                        println!("Failed to encode audio samples {:?}", e);
                    }
                };
            }
        });

        let audio_packets_handle = tokio::spawn(async move {
            while let Some(track) = audio_rx.recv().await {
                let _ = packet_sender.send(Packet::new(AudioPacket { track })).await;
            }
        });

        tokio::spawn(async move {
            select! {
                _ = microphone_handle => {
                }
                _ = audio_packets_handle => {
                }
                _ = stop_rx => {
                }
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), ClientError> {
        let stop_tx = match self.stop_tx.take() {
            Some(stop_tx) => stop_tx,
            None => return Ok(()),
        };

        match stop_tx.send(()) {
            Ok(_) => {
                return Ok(());
            }
            Err(_) => {
                return Err(ClientError::PoisonedLock);
            }
        }
    }

    fn get_codec(&self) -> Arc<Mutex<Codec>> {
        self.codec.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::codec::opus::OpusAudioCodec;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_audio_handler() {
        let mut audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 1)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(20);

        tokio::spawn(async move {
            let _ = audio_handler.start(mic_rx, packet_tx).await;
        });

        let audio_samples = vec![10.0; 480];
        tokio::spawn(async move {
            for _ in 0..10 {
                let _ = mic_tx.send(audio_samples.clone()).await;
            }
        });

        for _ in 0..10 {
            let packet = packet_rx.recv().await.unwrap();
            assert_eq!(packet.packet_id, 2);
            assert!(packet.data.len() > 75);
            assert!(packet.data.len() < 125);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn test_audio_handler_stop() {
        let mut audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 1)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(1);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(1);

        let _ = audio_handler.start(mic_rx, packet_tx).await;

        let audio_samples = vec![10.0; 480];
        tokio::spawn(async move {
            for _ in 0..10 {
                let _ = mic_tx.send(audio_samples.clone()).await;
                sleep(Duration::from_millis(10)).await;
                let _ = audio_handler.stop().await;
            }
        });

        sleep(Duration::from_millis(10)).await;
        let _ = packet_rx.recv().await.unwrap();
        for _ in 0..9 {
            sleep(Duration::from_millis(10)).await;
            assert!(packet_rx.recv().await.is_none());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_audio_handler_stereo() {
        let mut audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 2)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(20);

        let _ = audio_handler.start(mic_rx, packet_tx).await;

        let audio_samples = vec![10.0; 480 * 2];
        tokio::spawn(async move {
            for _ in 0..10 {
                let _ = mic_tx.send(audio_samples.clone()).await;
            }
        });

        for _ in 0..10 {
            let packet = packet_rx.recv().await.unwrap();
            assert_eq!(packet.packet_id, 2);
            assert!(packet.data.len() > 100);
            assert!(packet.data.len() < 225);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_audio_handler_invalid_packet() {
        let mut audio_handler = AudioProcessor::<OpusAudioCodec>::new().unwrap();
        {
            audio_handler
                .get_codec()
                .lock()
                .await
                .update(48000, 1)
                .unwrap();
        }

        let (mic_tx, mic_rx) = mpsc::channel::<Vec<f32>>(20);
        let (packet_tx, mut packet_rx) = mpsc::channel::<Packet>(20);

        let _ = audio_handler.start(mic_rx, packet_tx).await;

        let audio_samples = vec![10.0; 4890];
        tokio::spawn(async move {
            for _ in 0..3 {
                let _ = mic_tx.send(audio_samples.clone()).await;
            }
            let _ = mic_tx.send(vec![0.0; 480]).await;
        });

        for _ in 0..3 {
            assert!(packet_rx.try_recv().is_err());
        }

        sleep(Duration::from_millis(10)).await;
        let packet = packet_rx.try_recv().unwrap();
        assert_eq!(packet.packet_id, 2);
        assert!(packet.data.len() > 10);
        assert!(packet.data.len() < 15);

        for _ in 0..10 {
            assert!(packet_rx.try_recv().is_err());
        }
    }
}

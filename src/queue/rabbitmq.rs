use crate::common::types::SigningRequest;
use crate::error::TssError;
use anyhow::Result;
use futures_lite::stream::StreamExt;
use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
    ExchangeKind,
};

pub struct RabbitMQService {
    request_channel: Channel,
    request_exchange: String,
}

impl RabbitMQService {
    pub async fn new(uri: &str) -> Result<Self> {
        let conn = Connection::connect(uri, ConnectionProperties::default()).await?;
        let request_channel = conn.create_channel().await?;
        let result_channel = conn.create_channel().await?;
        let request_exchange = "signing_requests_exchange".to_string();
        let result_queue = "signing_results".to_string();

        request_channel
            .exchange_declare(
                &request_exchange,
                ExchangeKind::Fanout,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        result_channel
            .queue_declare(
                &result_queue,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Self {
            request_channel,
            request_exchange,
        })
    }

    pub async fn publish_signing_request(&self, request: &SigningRequest) -> Result<()> {
        let payload = serde_json::to_vec(request)?;
        self.request_channel
            .basic_publish(
                &self.request_exchange,
                "",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;

        Ok(())
    }

    pub async fn receive_signing_request(&self) -> Result<SigningRequest> {
        let queue_name = self
            .request_channel
            .queue_declare(
                "",
                QueueDeclareOptions {
                    exclusive: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?
            .name()
            .to_string();

        self.request_channel
            .queue_bind(
                &queue_name,
                &self.request_exchange,
                "",
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let mut consumer = self
            .request_channel
            .basic_consume(
                &queue_name,
                "signing_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        if let Some(delivery) = consumer.next().await {
            match delivery {
                Ok(delivery) => {
                    let request: SigningRequest = serde_json::from_slice(&delivery.data)?;
                    delivery.ack(BasicAckOptions::default()).await?;
                    Ok(request)
                }
                Err(err) => Err(err.into()),
            }
        } else {
            Err(TssError::QueueError("No message received".into()).into())
        }
    }
}

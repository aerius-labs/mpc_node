// use lapin::{
//     Channel, Connection, ConnectionProperties,
//     options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions},
//     types::FieldTable,
//     BasicProperties,
// };
// use futures::{Stream, StreamExt};
// use async_stream::try_stream;
// use crate::common::types::{SigningRequest, SigningResult};
// use crate::tss_error::TssError;
//
// pub struct RabbitMQService {
//     channel: Channel,
//     request_queue: String,
//     result_queue: String,
// }
//
// impl RabbitMQService {
//     pub async fn new(uri: &str) -> Result<Self, TssError> {
//         let conn = Connection::connect(uri, ConnectionProperties::default())
//             .await
//             .map_err(|err | TssError::QueueError(err.to_string()))?;
//         let channel = conn.create_channel().await.map_err(|err | TssError::QueueError(err.to_string()))?;
//
//         let service = Self {
//             channel,
//             request_queue: "signing_requests".into(),
//             result_queue: "signing_results".into(),
//         };
//
//         service.declare_queues().await?;
//
//         Ok(service)
//     }
//
//     async fn declare_queues(&self) -> Result<(), TssError> {
//         self.channel
//             .queue_declare(
//                 &self.request_queue,
//                 QueueDeclareOptions::default(),
//                 FieldTable::default(),
//             )
//             .await
//             .map_err(|err| TssError::QueueError(err.to_string()))?;
//
//         self.channel
//             .queue_declare(
//                 &self.result_queue,
//                 QueueDeclareOptions::default(),
//                 FieldTable::default(),
//             )
//             .await
//             .map_err(|err| TssError::QueueError(err.to_string()))?;
//
//         Ok(())
//     }
//
//     pub async fn publish_request(&self, request: &SigningRequest) -> Result<(), TssError> {
//         let payload = serde_json::to_vec(request).map_err(TssError::SerializationError)?;
//
//         self.channel
//             .basic_publish(
//                 "",
//                 &self.request_queue,
//                 BasicPublishOptions::default(),
//                 &payload,
//                 BasicProperties::default(),
//             )
//             .await
//             .map_err(|err| TssError::QueueError(err.to_string()))?;
//
//         Ok(())
//     }
//
//     pub async fn publish_result(&self, result: &SigningResult) -> Result<(), TssError> {
//         let payload = serde_json::to_vec(result).map_err(TssError::SerializationError)?;
//
//         self.channel
//             .basic_publish(
//                 "",
//                 &self.result_queue,
//                 BasicPublishOptions::default(),
//                 &payload,
//                 BasicProperties::default(),
//             )
//             .await
//             .map_err(|err| TssError::QueueError(err.to_string()))?;
//
//         Ok(())
//     }
//
//     pub async fn consume_requests(&self) -> Result<impl Stream<Item = Result<SigningRequest, TssError>>, TssError> {
//         let consumer = self.channel
//             .basic_consume(
//                 &self.request_queue,
//                 "signing_consumer",
//                 BasicConsumeOptions::default(),
//                 FieldTable::default(),
//             )
//             .await
//             .map_err(|err |TssError::QueueError(err.to_string()))?;
//
//         Ok(try_stream! {
//             for await delivery in consumer {
//                 let delivery = delivery.map_err(|err |TssError::QueueError(err.to_string()))?;
//                 let request: SigningRequest = serde_json::from_slice(&delivery.data)
//                     .map_err(TssError::SerializationError)?;
//
//                 delivery.ack(Default::default()).await.map_err(|err| TssError::QueueError(err.to_string()))?;
//                 yield request;
//             }
//         })
//     }
//
//     pub async fn consume_results(&self) -> Result<impl Stream<Item = Result<SigningResult, TssError>>, TssError> {
//         let consumer = self.channel
//             .basic_consume(
//                 &self.result_queue,
//                 "result_consumer",
//                 BasicConsumeOptions::default(),
//                 FieldTable::default(),
//             )
//             .await
//             .map_err(|err| TssError::QueueError(err.to_string()))?;
//
//         Ok(try_stream! {
//             for await delivery in consumer {
//                 let delivery = delivery.map_err(|err| TssError::QueueError(err.to_string()))?;
//                 let result: SigningResult = serde_json::from_slice(&delivery.data)
//                     .map_err(TssError::SerializationError)?;
//
//                 delivery.ack(Default::default()).await.map_err(|err| TssError::QueueError(err.to_string()))?;
//                 yield result;
//             }
//         })
//     }
//
//     pub async fn ping(&self) -> Result<(), TssError> {
//         // A simple ping could be declaring a temporary queue
//         self.channel
//             .queue_declare(
//                 "",
//                 QueueDeclareOptions {
//                     exclusive: true,
//                     auto_delete: true,
//                     ..Default::default()
//                 },
//                 FieldTable::default(),
//             )
//             .await
//             .map_err(|err| TssError::QueueError(err.to_string()))?;
//         Ok(())
//     }
// }

use crate::common::types::SigningRequest;
use crate::error::TssError;
use anyhow::Result;
use futures_lite::stream::StreamExt;
use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties,
    ExchangeKind,
};
use rocket::response;

pub struct RabbitMQService {
    request_channel: Channel,
    result_channel: Channel,
    request_exchange: String,
    result_queue: String,
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
            result_channel,
            request_exchange,
            result_queue,
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

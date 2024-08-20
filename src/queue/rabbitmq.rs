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

use lapin::{Connection, Channel, ConnectionProperties, options::*, types::FieldTable, message::DeliveryResult, BasicProperties};
use crate::common::types::SigningRequest;
use crate::error::TssError;
use futures_lite::stream::StreamExt;
use anyhow::Result;

pub struct RabbitMQService {
    channel: Channel,
    queue_name: String,
}

impl RabbitMQService {
    pub async fn new(uri: &str) -> Result<Self> {
        let conn = Connection::connect(uri, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await?;
        let queue_name = "signing_requests".to_string();

        channel
            .queue_declare(
                &queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        Ok(Self { channel, queue_name })
    }

    pub async fn publish_signing_request(&self, request: &SigningRequest) -> Result<()> {
        let payload = serde_json::to_vec(request)?;
        self.channel
            .basic_publish(
                "",
                &self.queue_name,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;
        Ok(())
    }

    pub async fn receive_signing_request(&self) -> Result<SigningRequest> {
        let mut consumer = self.channel
            .basic_consume(
                &self.queue_name,
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
// Dotlanth
// Copyright (C) 2025 Synerthink

use super::types::GqlWebSocketMessage as WebSocketMessage;
use crate::vm::VmClient;
use crate::websocket::WebSocketManager;
use async_graphql::futures_util::stream::{Stream, StreamExt};
use async_graphql::{Context, Result as GqlResult, Subscription};
use std::pin::Pin;
use std::sync::Arc;

pub struct SubscriptionRoot {
    vm: VmClient,
}

impl SubscriptionRoot {
    pub fn new(vm: VmClient) -> Self {
        Self { vm }
    }
}

#[Subscription]
impl SubscriptionRoot {
    async fn events(&self, ctx: &Context<'_>, event_type: String) -> Pin<Box<dyn Stream<Item = WebSocketMessage> + Send>> {
        // For now, use a simple channel bridged from the WebSocketManager's broadcaster if available
        // This ensures real-time updates; in a production system we'd hook to actual VM streams
        let manager = ctx.data_opt::<Arc<WebSocketManager>>().cloned();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        if let Some(mgr) = manager {
            let mut receiver = mgr.subscribe(&event_type);
            tokio::spawn(async move {
                while let Ok(msg) = receiver.recv().await {
                    let _ = tx.send(WebSocketMessage::from(msg));
                }
            });
        }
        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(|m| m);
        Box::pin(stream)
    }
}

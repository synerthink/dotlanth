// Dotlanth
// Copyright (C) 2025 Synerthink

use std::sync::Arc;

use crate::auth::AuthService;
use crate::db::DatabaseClient;
use crate::vm::VmClient;
use crate::websocket::WebSocketManager;
use async_graphql::Schema;
use async_graphql::extensions::Logger;
use std::sync::Arc as StdArc;

use super::{mutation::MutationRoot, query::QueryRoot, subscription::SubscriptionRoot};

pub type AppSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

pub fn build_schema(auth: Arc<tokio::sync::Mutex<AuthService>>, db: DatabaseClient, vm: VmClient, ws_manager: StdArc<WebSocketManager>) -> AppSchema {
    Schema::build(QueryRoot::default(), MutationRoot::default(), SubscriptionRoot::new(vm.clone()))
        .limit_complexity(2000)
        .limit_depth(20)
        .data(auth)
        .data(db)
        .data(vm)
        .data(ws_manager)
        .extension(Logger)
        .finish()
}

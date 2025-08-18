// Dotlanth
// Copyright (C) 2025 Synerthink

use super::types::{GqlApiVersion, GqlCollection, GqlDocument, GqlDocumentList, GqlSearchResults};
use crate::db::DatabaseClient;
use crate::models::SearchResults;
use crate::vm::VmClient;
use async_graphql::{Context, Object, Result as GqlResult};

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn api_version(&self) -> GqlApiVersion {
        GqlApiVersion {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build: format!("{}+{}", env!("CARGO_PKG_VERSION"), option_env!("GIT_HASH").unwrap_or("unknown")),
            features: vec!["graphql".to_string(), "subscriptions".to_string()],
        }
    }

    async fn collections(&self, ctx: &Context<'_>) -> GqlResult<Vec<GqlCollection>> {
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let cols = db.list_collections().await?;
        Ok(cols.into_iter().map(GqlCollection::from).collect())
    }

    async fn documents(&self, ctx: &Context<'_>, collection: String, page: Option<u32>, page_size: Option<u32>) -> GqlResult<GqlDocumentList> {
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let list = db.get_documents(&collection, page.unwrap_or(1), page_size.unwrap_or(20)).await?;
        Ok(list.into())
    }

    async fn document(&self, ctx: &Context<'_>, collection: String, id: String) -> GqlResult<GqlDocument> {
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let d = db.get_document(&collection, &id).await?;
        Ok(d.into())
    }

    async fn vm_status(&self, ctx: &Context<'_>) -> GqlResult<serde_json::Value> {
        let vm = ctx.data_unchecked::<VmClient>().clone();
        Ok(vm.get_vm_status().await?)
    }

    async fn search_documents(&self, ctx: &Context<'_>, collection: String, q: String, limit: Option<u32>, offset: Option<u32>) -> GqlResult<GqlSearchResults> {
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let r = db.search_documents(&collection, &q, limit, offset).await?;
        Ok(r.into())
    }

    async fn architectures(&self, ctx: &Context<'_>) -> GqlResult<Vec<String>> {
        let vm = ctx.data_unchecked::<VmClient>().clone();
        Ok(vm.get_architectures().await?)
    }
}

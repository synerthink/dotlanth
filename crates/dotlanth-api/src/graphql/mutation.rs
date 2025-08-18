// Dotlanth
// Copyright (C) 2025 Synerthink

use super::guards::ClaimsExt;
use super::types::{GqlCollection, GqlCreateDocumentResponse, GqlDeployDotInput, GqlDeployDotResponse, GqlDocument, GqlExecuteDotInput, GqlExecuteDotResponse};
use crate::auth::Claims;
use crate::db::DatabaseClient;
use crate::models;
use crate::vm::VmClient;
use async_graphql::{Context, Object, Result as GqlResult};

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_collection(&self, ctx: &Context<'_>, name: String) -> GqlResult<GqlCollection> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["write:documents"])?;
        }
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let c = db.create_collection(&name).await?;
        Ok(c.into())
    }

    async fn delete_collection(&self, ctx: &Context<'_>, name: String) -> GqlResult<bool> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["delete:documents"])?;
        }
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        db.delete_collection(&name).await?;
        Ok(true)
    }

    async fn create_document(&self, ctx: &Context<'_>, collection: String, content: serde_json::Value) -> GqlResult<GqlCreateDocumentResponse> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["write:documents"])?;
        }
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let r = db.create_document(&collection, content).await?;
        Ok(r.into())
    }

    async fn update_document(&self, ctx: &Context<'_>, collection: String, id: String, content: serde_json::Value) -> GqlResult<GqlDocument> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["write:documents"])?;
        }
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        let d = db.update_document(&collection, &id, content).await?;
        Ok(d.into())
    }

    async fn delete_document(&self, ctx: &Context<'_>, collection: String, id: String) -> GqlResult<bool> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["delete:documents"])?;
        }
        let db = ctx.data_unchecked::<DatabaseClient>().clone();
        db.delete_document(&collection, &id).await?;
        Ok(true)
    }

    async fn deploy_dot(&self, ctx: &Context<'_>, input: GqlDeployDotInput) -> GqlResult<GqlDeployDotResponse> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["deploy:dots"])?;
        }
        let vm = ctx.data_unchecked::<VmClient>().clone();
        let resp = vm.deploy_dot(input.into()).await?;
        Ok(resp.into())
    }

    async fn execute_dot(&self, ctx: &Context<'_>, dot_id: String, input: GqlExecuteDotInput) -> GqlResult<GqlExecuteDotResponse> {
        if let Some(claims) = ctx.data_opt::<Claims>() {
            claims.require_permissions(&["execute:dots"])?;
        }
        let vm = ctx.data_unchecked::<VmClient>().clone();
        let resp = vm.execute_dot(&dot_id, input.into()).await?;
        Ok(resp.into())
    }
}

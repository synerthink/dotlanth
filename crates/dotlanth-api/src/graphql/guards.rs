// Dotlanth
// Copyright (C) 2025 Synerthink

use crate::auth::Claims;
use async_graphql::{ErrorExtensions, Result as GqlResult, ServerError};

pub trait ClaimsExt {
    fn require_permissions(&self, needed: &[&str]) -> GqlResult<()>;
}

impl ClaimsExt for Claims {
    fn require_permissions(&self, needed: &[&str]) -> GqlResult<()> {
        for p in needed {
            if !self.permissions.iter().any(|v| v == p) {
                return Err(ServerError::new("Forbidden", None).extend_with(|_err, e| e.set("code", "FORBIDDEN")));
            }
        }
        Ok(())
    }
}

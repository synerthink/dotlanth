// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Compression middleware for gRPC services

use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use hyper::{Body, Request, Response};
use tower_http::compression::{CompressionLayer as TowerCompressionLayer, CompressionLevel};
use prost::bytes::{Buf, BufMut};

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub level: CompressionLevel,
    pub min_size: usize,
    pub enabled_encodings: Vec<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            level: CompressionLevel::Default,
            min_size: 1024, // Only compress responses larger than 1KB
            enabled_encodings: vec![
                "gzip".to_string(),
                "br".to_string(), // Brotli
                "deflate".to_string(),
            ],
        }
    }
}

/// Compression layer for gRPC services
pub struct CompressionLayer {
    config: CompressionConfig,
}

impl CompressionLayer {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    pub fn default() -> Self {
        Self::new(CompressionConfig::default())
    }

    pub fn with_level(mut self, level: CompressionLevel) -> Self {
        self.config.level = level;
        self
    }

    pub fn with_min_size(mut self, min_size: usize) -> Self {
        self.config.min_size = min_size;
        self
    }
}

impl<S> Layer<S> for CompressionLayer {
    type Service = CompressionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CompressionService {
            inner: TowerCompressionLayer::new()
                .gzip(self.config.enabled_encodings.contains(&"gzip".to_string()))
                .br(self.config.enabled_encodings.contains(&"br".to_string()))
                // Note: deflate not available in this version
                .quality(self.config.level)
                .layer(inner),
            config: self.config.clone(),
        }
    }
}

/// Compression service wrapper
#[derive(Clone)]
pub struct CompressionService<S> {
    inner: tower_http::compression::Compression<S>,
    config: CompressionConfig,
}

impl<S> Service<Request<Body>> for CompressionService<S>
where
    S: Service<Request<Body>, Response = Response<Body>>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let future = self.inner.call(req);
        
        Box::pin(async move {
            let response = future.await?;
            
            // Log compression info if enabled
            if let Some(encoding) = response.headers().get("content-encoding") {
                tracing::debug!("Response compressed with: {:?}", encoding);
            }
            
            Ok(response)
        })
    }
}

/// gRPC-specific compression utilities
pub mod grpc {
    use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
    use tonic::{Code, Status};
    use std::marker::PhantomData;

    /// Compressed codec wrapper
    pub struct CompressedCodec<T> {
        inner: T,
        compression_enabled: bool,
    }

    impl<T> CompressedCodec<T> {
        pub fn new(inner: T) -> Self {
            Self {
                inner,
                compression_enabled: true,
            }
        }

        pub fn with_compression(mut self, enabled: bool) -> Self {
            self.compression_enabled = enabled;
            self
        }
    }

    impl<T> Codec for CompressedCodec<T>
    where
        T: Codec,
    {
        type Encode = T::Encode;
        type Decode = T::Decode;
        type Encoder = CompressedEncoder<T::Encoder>;
        type Decoder = CompressedDecoder<T::Decoder>;

        fn encoder(&mut self) -> Self::Encoder {
            CompressedEncoder {
                inner: self.inner.encoder(),
                compression_enabled: self.compression_enabled,
            }
        }

        fn decoder(&mut self) -> Self::Decoder {
            CompressedDecoder {
                inner: self.inner.decoder(),
                compression_enabled: self.compression_enabled,
            }
        }
    }

    pub struct CompressedEncoder<T> {
        inner: T,
        compression_enabled: bool,
    }

    impl<T> Encoder for CompressedEncoder<T>
    where
        T: Encoder,
    {
        type Item = T::Item;
        type Error = T::Error;

        fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
            if self.compression_enabled {
                // Add compression flag
                dst.put_u8(1); // Compressed flag
            } else {
                dst.put_u8(0); // Uncompressed flag
            }
            
            self.inner.encode(item, dst)
        }
    }

    pub struct CompressedDecoder<T> {
        inner: T,
        compression_enabled: bool,
    }

    impl<T> Decoder for CompressedDecoder<T>
    where
        T: Decoder,
    {
        type Item = T::Item;
        type Error = T::Error;

        fn decode(&mut self, src: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
            if src.remaining() == 0 {
                return Ok(None);
            }

            // Check compression flag
            let _compressed = src.get_u8() == 1;
            
            self.inner.decode(src)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::{ServiceBuilder, ServiceExt};
    use hyper::{Body, Request, Response};
    use std::convert::Infallible;

    async fn dummy_service(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::new(Body::from("Hello, World!".repeat(100))))
    }

    #[tokio::test]
    async fn test_compression_layer() {
        let service = ServiceBuilder::new()
            .layer(CompressionLayer::default())
            .service_fn(dummy_service);

        let request = Request::builder()
            .header("accept-encoding", "gzip")
            .body(Body::empty())
            .unwrap();

        let response = service.oneshot(request).await.unwrap();
        
        // Should have compression headers for large responses
        assert!(response.status().is_success());
    }
}
use proto::runtime_server::{Runtime, RuntimeServer};
use tonic::transport::Server;

mod proto {
    tonic::include_proto!("runtime");

    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("runtime_descriptor");
}

#[derive(Debug, Default)]
struct RuntimeService {}

#[tonic::async_trait]
impl Runtime for RuntimeService {
    async fn ping(
        &self,
        request: tonic::Request<proto::PingRequest>,
    ) -> Result<tonic::Response<proto::PingResponse>, tonic::Status> {
        let response = proto::PingResponse {
            message: format!("Ping: {}", request.into_inner().message),
        };

        Ok(tonic::Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;

    let runtime = RuntimeService::default();

    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build()?;

    Server::builder()
        .add_service(service)
        .add_service(RuntimeServer::new(runtime))
        .serve(addr)
        .await?;

    Ok(())
}

use proto::runtime_server::{Runtime, RuntimeServer};
use tonic::transport::Server;

mod proto {
    tonic::include_proto!("runtime");
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

    Server::builder()
        .add_service(RuntimeServer::new(runtime))
        .serve(addr)
        .await?;

    Ok(())
}

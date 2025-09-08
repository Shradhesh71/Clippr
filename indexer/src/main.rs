use yellowstone::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::geyser_client::GeyserClient;
pub mod yellowstone;

#[tokio::main]
async fn main() {   
    let client = GeyserGrpcClient::new(HealthClient::new(), GeyserClient::new());
    client.health_check().await;

    


}

use tokio::runtime::Runtime;

mod network;

use network::Network;

fn main() {
    tracing_subscriber::fmt::init();

    Runtime::new().unwrap().block_on(async {
        let network = Network::new("target/db").await.unwrap();
        let identity_id = network.generate_identity().await.unwrap();
        network.start(&identity_id).await.unwrap();
    });
}

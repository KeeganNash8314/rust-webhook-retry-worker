use std::env;
use webhook_retry_queue::QueueClient;

fn main() -> Result<(), String> {
    let client = QueueClient::from_env()?;
    let target = env::var("WEBHOOK_URL")
        .map_err(|_| "set WEBHOOK_URL to the delivery endpoint".to_string())?;
    let queue = env::var("QUEUE").map_err(|_| "set QUEUE to the queue name".to_string())?;

    let queued = client.publish(&queue, &target, "webhook-delivery-001")?;
    println!("queued: {queued}");

    let messages = client.consume(&queue, 1, 60)?;
    println!("leased: {messages}");

    if let Ok(message_id) = env::var("MESSAGE_ID") {
        let acknowledged = client.ack(&queue, &message_id)?;
        println!("acknowledged: {acknowledged}");
    }
    Ok(())
}

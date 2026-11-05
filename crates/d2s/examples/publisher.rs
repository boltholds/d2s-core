use d2s::{Message, PHASE0_TOPIC, Publisher};
use std::env;
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let domain: u32 = args.next().unwrap_or_else(|| "10".into()).parse()?;
    let count: u32 = args.next().unwrap_or_else(|| "5".into()).parse()?;

    let mut publisher = Publisher::new(domain, PHASE0_TOPIC)?;
    println!("Publisher ready on DDS domain {domain}; waiting for reader...");

    if !publisher.wait_for_readers(Duration::from_secs(15)).await {
        return Err("No matching DDS reader discovered within 15 seconds".into());
    }

    for index in 1..=count {
        let message = Message {
            index,
            text: format!("Hello from D2S Rust publisher #{index}"),
        };
        publisher.publish(&message)?;
        println!("SENT [{}] {}", message.index, message.text);
        sleep(Duration::from_millis(200)).await;
    }

    // This is NOT a DDS acknowledgment guarantee. Give the demo a small
    // grace interval before dropping the native writer.
    sleep(Duration::from_millis(500)).await;
    Ok(())
}

use d2s::{PHASE0_TOPIC, Subscriber};
use std::env;
use std::error::Error;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let domain: u32 = args.next().unwrap_or_else(|| "10".into()).parse()?;
    let count: u32 = args.next().unwrap_or_else(|| "5".into()).parse()?;

    let mut subscriber = Subscriber::new(domain, PHASE0_TOPIC)?;
    println!("Subscriber listening on DDS domain {domain}...");

    for _ in 0..count {
        match subscriber.receive_timeout(Duration::from_secs(15)).await? {
            Some(message) => println!("RECEIVED [{}] {}", message.index, message.text),
            None => return Err("Timed out waiting for DDS sample".into()),
        }
    }

    Ok(())
}

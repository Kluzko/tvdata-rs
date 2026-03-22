use tvdata_rs::{CalendarWindowRequest, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let events = client
        .ipo_calendar(&CalendarWindowRequest::trailing("america", 30).limit(10))
        .await?;

    println!("events: {}", events.len());
    for event in events.iter().take(5) {
        println!(
            "{:?} {:?} {:?}",
            event.offer_date, event.instrument.ticker, event.offer_price_usd
        );
    }

    Ok(())
}

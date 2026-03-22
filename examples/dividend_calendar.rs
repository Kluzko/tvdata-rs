use tvdata_rs::{DividendCalendarRequest, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let events = client
        .dividend_calendar(&DividendCalendarRequest::upcoming("america", 14).limit(10))
        .await?;

    println!("events: {}", events.len());
    for event in events.iter().take(5) {
        println!(
            "{:?} {:?} {:?}",
            event.effective_date, event.instrument.ticker, event.amount
        );
    }

    Ok(())
}

use tvdata_rs::{CalendarWindowRequest, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let events = client
        .earnings_calendar(&CalendarWindowRequest::upcoming("america", 7).limit(10))
        .await?;

    println!("events: {}", events.len());
    for event in events.iter().take(5) {
        println!(
            "{:?} {:?} {:?}",
            event.release_at, event.instrument.ticker, event.eps_forecast_next_fq
        );
    }

    Ok(())
}

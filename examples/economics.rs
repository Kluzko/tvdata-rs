use tvdata_rs::{EconomicCalendarRequest, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let response = client
        .economic_calendar(&EconomicCalendarRequest::upcoming(7))
        .await?;

    println!("status: {:?}", response.status);
    println!("events: {}", response.events.len());
    for event in response.events.iter().take(5) {
        println!(
            "{:?} {:?} {:?} {:?}",
            event.date, event.country, event.indicator, event.actual
        );
    }

    Ok(())
}

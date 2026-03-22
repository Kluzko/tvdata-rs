use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let series = client
        .history(&HistoryRequest::max("NASDAQ:AAPL", Interval::Day1))
        .await?;

    println!("max bars: {}", series.bars.len());
    Ok(())
}

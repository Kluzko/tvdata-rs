use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let series = client
        .download_history(["NASDAQ:AAPL", "NASDAQ:MSFT"], Interval::Day1, 10)
        .await?;

    for item in &series {
        println!("{} -> {} bars", item.symbol, item.bars.len());
    }

    let mapped = client
        .download_history_map(["BINANCE:BTCUSDT", "FX:EURUSD"], Interval::Hour1, 24)
        .await?;

    println!("downloaded {} series into map", mapped.len());
    Ok(())
}

use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let crypto = client.crypto();

    let quote = crypto.quote("BINANCE:BTCUSDT").await?;
    let overview = crypto.overview("BINANCE:BTCUSDT").await?;
    let gainers = crypto.top_gainers(5).await?;

    println!("quote: {quote:#?}");
    println!("overview: {overview:#?}");
    println!("top gainers: {gainers:#?}");

    Ok(())
}

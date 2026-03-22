use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let equity = client.equity();

    let quote = equity.quote("NASDAQ:AAPL").await?;
    let analyst = equity.analyst_summary("NASDAQ:AAPL").await?;
    let targets = equity.price_targets("NASDAQ:AAPL").await?;
    let earnings = equity.earnings_calendar("NASDAQ:AAPL").await?;
    let overview = equity.overview("NASDAQ:AAPL").await?;
    let gainers = equity.top_gainers("america", 5).await?;

    println!("quote: {quote:#?}");
    println!("analyst: {analyst:#?}");
    println!("targets: {targets:#?}");
    println!("earnings: {earnings:#?}");
    println!("overview: {overview:#?}");
    println!("top gainers: {gainers:#?}");

    Ok(())
}

use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let forex = client.forex();

    let quote = forex.quote("FX:EURUSD").await?;
    let overview = forex.overview("FX:EURUSD").await?;
    let active = forex.most_active(5).await?;

    println!("quote: {quote:#?}");
    println!("overview: {overview:#?}");
    println!("most active: {active:#?}");

    Ok(())
}

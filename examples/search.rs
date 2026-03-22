use tvdata_rs::{Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let response = client.search_equities_response("AAPL").await?;

    println!("remaining symbols: {}", response.symbols_remaining);
    for hit in response.hits.iter().take(5) {
        println!(
            "{} {:?} {:?} {:?}",
            hit.symbol, hit.exchange, hit.instrument_type, hit.isin
        );
    }

    Ok(())
}

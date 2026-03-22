use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let query = ScanQuery::new()
        .market("america")
        .tickers(["NASDAQ:AAPL"])
        .select(fields::quote_snapshot())
        .push_column(fields::technical::RSI)
        .push_column(fields::technical::MACD)
        .push_column(fields::analyst::PRICE_TARGET_AVERAGE);

    let response = client.scan(&query).await?;
    let first_row = &response.rows[0];
    let record = first_row.as_record(&query.columns);

    println!("{record:#?}");
    Ok(())
}

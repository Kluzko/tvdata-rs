use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let query = ScanQuery::new()
        .market("america")
        .select([fields::core::NAME, fields::price::CLOSE]);

    let report = client.validate_scan_query(&query).await?;
    println!("strictly supported: {}", report.is_strictly_supported());

    let response = client.scan_validated(&query).await?;
    println!("rows: {}", response.rows.len());
    Ok(())
}

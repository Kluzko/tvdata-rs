use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let query = ScanQuery::new()
        .markets(["america", "crypto"])
        .select([fields::price::CLOSE, fields::fundamentals::MARKET_CAP_BASIC]);

    let (filtered, report) = client.filter_scan_query(&query).await?;
    println!("strictly supported: {}", report.is_strictly_supported());
    println!("kept columns: {:?}", report.filtered_column_names());
    println!(
        "dropped columns: {:?}",
        report.strict_violation_column_names()
    );
    println!("filtered column count: {}", filtered.columns.len());

    let response = client.scan_supported(&query).await?;
    println!("rows: {}", response.rows.len());

    Ok(())
}

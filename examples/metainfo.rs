use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let metainfo = client.metainfo("america").await?;

    println!("financial currency: {:?}", metainfo.financial_currency);
    println!("field count: {}", metainfo.fields.len());
    println!(
        "supports price_target_average: {}",
        metainfo.supports_field("price_target_average")
    );

    Ok(())
}

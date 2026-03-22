use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let history = client
        .history(
            &HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 10)
                .session(TradingSession::Regular),
        )
        .await?;

    for bar in history.bars {
        println!(
            "{} o={} h={} l={} c={} v={:?}",
            bar.time, bar.open, bar.high, bar.low, bar.close, bar.volume
        );
    }

    Ok(())
}

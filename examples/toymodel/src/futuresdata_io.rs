use quantlib::instruments::futures::Futures;
use quantlib::currency::Currency;
use time::macros::datetime;
use anyhow::{Result, Context};
use serde_json::{
    from_str,
    to_string,
};
use tracing::info;
use std::fs::{
    write,
    read_to_string,
};

pub fn futuresdata_io() -> Result<()> {
    let futures1 = Futures::new(
        350.0,
        datetime!(2021-01-01 17:00:00 +09:00),
        datetime!(2022-01-01 17:00:00 +09:00),
        datetime!(2022-01-01 17:00:00 +09:00),
        datetime!(2022-01-01 17:00:00 +09:00),
        250_000.0,
        Currency::KRW,
        Currency::KRW,
        "KOSPI2".to_string(),
        "KOSPI2 fut1".to_string(),
        "165AAA".to_string(),
    );

    let futures2 = Futures::new(
        5_000.0,
        datetime!(2021-01-01 17:00:00 +09:00),
        datetime!(2022-01-01 17:00:00 +09:00),
        datetime!(2022-01-01 17:00:00 +09:00),
        datetime!(2022-01-01 17:00:00 +09:00),
        50.0,
        Currency::USD,
        Currency::USD,
        "SPX".to_string(),
        "SPX fut1".to_string(),
        "ESF22".to_string(),
    );

    let futdate_vec = vec![futures1, futures2];

    let json = to_string(&futdate_vec)
        .context("Failed to serialize Vec<Futures> to JSON")?;

    write("json_data/futuresdata.json", &json)
        .context("Failed to write JSON to file")?;

    // re-read the file
    let json = read_to_string("json_data/futuresdata.json")
        .context("Failed to read JSON from file")?;

    let res: Vec<Futures> = from_str(&json)
        .context("Failed to deserialize JSON to Futures")?;

    info!("{:?}", res);
    Ok(())
}
use core::fmt;
use std::collections::HashMap;
use serde::{Deserialize};
use std::env;
use clap::{Arg, App};
use csv::Writer;
use log::{info, debug};
use log4rs;

#[derive(Deserialize, Debug)]
struct Quote {
    price: f64,
    percent_change_7d: f64,
    volume_24h: f64,
    market_cap: f64,
}

#[derive(Deserialize, Debug)]
struct Currency {
    id: i32,
    name: String,
    symbol: String,
    slug: String,
    quote: HashMap<String, Quote>,
}


#[derive(Deserialize, Debug)]
struct CMCResponse {
    data: HashMap<String, Currency>,
}

#[derive(Debug)]
enum OneError {
    CSV(csv::Error),
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl std::error::Error for OneError {}

impl fmt::Display for OneError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OneError::CSV(err) => write!(f, "Error while writing the CSV file {}", err),
            OneError::IO(err) => write!(f, "Error while flushing the file {}", err),
            OneError::Reqwest(err) => write!(f, "Error while fetching data {}", err),
        }
    }
}

impl From<reqwest::Error> for OneError {
    fn from(err: reqwest::Error) -> OneError {
        OneError::Reqwest(err)
    }
}

impl From<csv::Error> for OneError {
    fn from(err: csv::Error) -> OneError {
        OneError::CSV(err)
    }
}

impl From<std::io::Error> for OneError {
    fn from(err: std::io::Error) -> OneError {
        OneError::IO(err)
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv()?;
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let matches = App::new("Cryptocurrency prices")
        .version("1.0")
        .author("Vlad Lytvynenko. <sir.sagramor@gmail.com>")
        .about("Gets prices of given cryptocurrencies")
        .arg(Arg::with_name("currencies")
            .long("currencies")
            .min_values(1)
            .required(true))
        .get_matches();

    let currencies = matches
        .value_of("currencies")
        .expect("There are no currencies were passes");

    debug!("Querying the following currencies: {:?}", currencies);

    let client = reqwest::Client::new();
    let api_key = env::var("CMS_API_KEY").expect("CMS_API_KEY key not set");
    let resp = client
        .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest")
        .header("X-CMC_PRO_API_KEY", api_key)
        .query(&[("symbol", currencies.to_string())])
        .send()
        .await?;

    let status = resp.status();
    match status {
        reqwest::StatusCode::OK => {
            let resp: CMCResponse = resp.json().await?;
            let mut wtr = Writer::from_path("out.csv")?;
            wtr.write_record(&["name", "symbol", "price", "percent_change_7d"])?;
            for currency in resp.data.values() {
                wtr.write_record(&[
                    &currency.name,
                    &currency.symbol,
                    &currency.quote["USD"].price.to_string(),
                    &currency.quote["USD"].percent_change_7d.to_string()]
                )?;
            }
            wtr.flush()?;
        }
        _ => {
            info!("Status: {}\nResponse Body: {}", status, resp.text().await?);
        }
    }

    info!("Queried {} and wrote CSV file", currencies);

    Ok(())
}

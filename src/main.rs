pub mod lib;

use clap::Clap;
use futures::stream::iter;
use futures::StreamExt;
use lib::api::get_card_prices::price_record;
use lib::{get_record, get_record_from_reader, Records};
use reqwest::Client;
use std::str::FromStr;
use url::ParseError;

#[derive(Clap, Debug)]
enum ArbitrationStrategy {
    MinValue,
    MaxValue,
}

impl FromStr for ArbitrationStrategy {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Min" | "MinValue" => Ok(ArbitrationStrategy::MinValue),
            "Max" | "MaxValue" => Ok(ArbitrationStrategy::MaxValue),
            _ => Ok(ArbitrationStrategy::MinValue),
        }
    }
}

impl Into<lib::ArbitrationStrategy> for ArbitrationStrategy {
    fn into(self) -> lib::ArbitrationStrategy {
        match self {
            ArbitrationStrategy::MinValue => lib::ArbitrationStrategy::MinValue,
            ArbitrationStrategy::MaxValue => lib::ArbitrationStrategy::MaxValue,
        }
    }
}

#[derive(Clap, Debug)]
struct Opts {
    #[clap(short, about = "Path to input file, otherwise input comes from stdin")]
    file: Option<String>,
    #[clap(short, about = "Path to output file, otherwise output goes to stdout")]
    out: Option<String>,
    #[clap(long, about = "Prints total value of cards to stdout")]
    print_total: bool,
    #[clap(
        short,
        default_value = "Min",
        about = r#"Arbitration strategy when result is ambiguous. 'Min' or 'MinValue' to pick cheapest option. 'Max' or 'MaxValue' to pick most expensive option."#
    )]
    arbitration_strategy: ArbitrationStrategy,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    let client = Client::default();
    let records = match opts.file {
        None => get_record_from_reader(std::io::stdin()),
        Some(filename) => get_record(filename.as_str()).unwrap(),
    };

    let arb_strategy = opts.arbitration_strategy.into();
    let records: Records = iter(records)
        .then(|x| price_record(x, &client, arb_strategy))
        .collect()
        .await;

    let writer: Box<dyn std::io::Write> = match opts.out {
        None => Box::new(std::io::stdout()),
        Some(filename) => Box::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(filename)
                .unwrap(),
        ),
    };

    let buf_writer = std::io::BufWriter::new(writer);

    let mut writer = csv::Writer::from_writer(buf_writer);

    for record in records.iter() {
        writer.serialize(record).unwrap();
    }

    if opts.print_total {
        let total_value = records.iter().fold(0f32, |acc, record| {
            let count = record.count.unwrap_or(1) as f32;
            acc + record.price.unwrap() * count
        });
        println!("total value: ${}", total_value)
    }
}

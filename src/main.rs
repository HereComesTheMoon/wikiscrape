use reqwest::Client;
use reqwest::Method;

use serde::{Serialize};
use serde_json::Value;

use csv;
use std::fs::File;
use std::io::BufWriter;
use futures::{stream, StreamExt};
use futures::future;

const USER_AGENT: &str = "JustTestingForNow/0.0 (testing@protonmail.ch)";
const LENGTH_ID_PREFIX: usize = "http://www.wikidata.org/entity/".len();
const LENGTH_ARTICLE_PREFIX: usize = "https://en.wikipedia.org/wiki/".len();
const CONCURRENT_REQUESTS: usize = 10;

#[derive(Debug, Clone)]
struct Country {
    name: String,
    id: String,
}

#[derive(Debug, Clone)]
struct City {
    country: String, // Human-readable country name
    name: String,    // Human-readable name
    // title: String, // https://en.wikipedia.org/wiki/<title>
    id: String, // Wikidata ID, always Q<some number>
}

#[derive(Serialize, Debug)]
struct Record {
    country: String,     // Human-readable country name
    name: String,        // Human-readable name
    description: String, // Summary paragraph from wikipedia
    id: String,          // Wikidata ID, always Q<some number>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let res = query_countries().await?;

    let cities = query_cities(res[0].clone()).await?;
    // let city_names = cities.iter().map(|city| &city.name);

    // // let client = Client::new();
    // let bodies = stream::iter(cities)
    //     .map(|city| {
    //         // let client = &client;
    //         async {
    //             get_record(city).await.unwrap()
    //         }
    //         // async move {
    //         //     let resp = client.get(url).send().await?;
    //         //     resp.bytes().await
    //         // }
    //     })
    //     .buffer_unordered(CONCURRENT_REQUESTS);

    // futures::future::join_all(bodies);

    let zukunft = stream::iter(cities.into_iter())
        .map(|city| async {
                get_record(city).await.unwrap()
            }
        )
        .buffer_unordered(CONCURRENT_REQUESTS);
    // futures::future::join_all(zukunft).await;
    // zukunft.for_each(|b| async move {
    //     println!("Res: {:?}", b);
    // }).await;

    let mut results: Vec<Record> = zukunft.collect::<Vec<_>>().await;

    // bodies
    //     .for_each(|b| async {
    //         match b {
    //             Ok(res) => results.push(res),
    //             Err(e) => println!("Oh no! {:?}", e),
    //         }
    //     });

    // println!("{:?}", cities);
    // for entry in cities {
    //     println!("{:?}", entry);
    //     let summary = get_record(entry).await?;
    //     println!("{:?}", summary);
    // }
    Ok(())
}

async fn query_countries() -> Result<Vec<Country>, Box<dyn std::error::Error>> {
    let query = "
        SELECT DISTINCT ?entity ?entityLabel WHERE {
          ?entity wdt:P31 wd:Q6256 . 
          ?article schema:about ?entity .
          ?article schema:isPartOf <https://en.wikipedia.org/>.
          FILTER NOT EXISTS {?entity wdt:P31 wd:Q3024240}
          FILTER NOT EXISTS {?entity wdt:P31 wd:Q28171280}
          OPTIONAL { ?entity wdt:P576 ?dissolved } .
          FILTER (!BOUND(?dissolved)) 
          SERVICE wikibase:label { bd:serviceParam wikibase:language \"en\" . }
        }
        ORDER BY ?entityLabel
    ";
    let res = Client::new()
        .request(Method::GET, "https://query.wikidata.org/sparql?")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .query(&[("query", query), ("format", "json")])
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    let res: &Vec<serde_json::Value> = res
        .get("results")
        .unwrap()
        .get("bindings")
        .unwrap()
        .as_array()
        .unwrap();

    let countries = res
        .iter()
        .map(|val| Country {
            name: val
                .get("entityLabel")
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned(),
            id: val
                .get("entity")
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap()[LENGTH_ID_PREFIX..]
                .to_owned(),
        })
        .collect();

    Ok(countries)
}

async fn query_cities(country: Country) -> Result<Vec<City>, Box<dyn std::error::Error>> {
    let query = format!(
        "
    SELECT DISTINCT ?entity ?article WHERE {{
      ?article schema:about ?entity .
      ?article schema:isPartOf <https://en.wikipedia.org/>.
      ?entity (wdt:P31/(wdt:P279*)) wd:Q515.
      ?entity wdt:P17 wd:{}.
    }}
    ",
        country.id
    );

    let res = Client::new()
        .request(Method::GET, "https://query.wikidata.org/sparql?")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .query(&[("query", query), ("format", "json".to_owned())])
        .send()
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;

    let res: &Vec<serde_json::Value> = res
        .get("results")
        .unwrap()
        .get("bindings")
        .unwrap()
        .as_array()
        .unwrap();

    let cities = res
        .iter()
        .map(|val| City {
            name: val
                .get("article")
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap()[LENGTH_ARTICLE_PREFIX..]
                .to_owned(),
            id: val
                .get("entity")
                .unwrap()
                .get("value")
                .unwrap()
                .as_str()
                .unwrap()[LENGTH_ID_PREFIX..]
                .to_owned(),
            country: country.name.clone(),
        })
        .collect();

    Ok(cities)
}

async fn get_record(city: City) -> Result<Record, Box<dyn std::error::Error>> {
    let query: String =
        "https://en.wikipedia.org/api/rest_v1/page/summary/".to_owned() + &city.name;
    let res = loop {
        let request = Client::new()
            .request(Method::GET, &query)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?
            .error_for_status();

        match request {
            Ok(res) => break res,
            Err(err) => println!("Error fetching {:?}: {:?}", city.name, err),
        }
    };

    let res = res.json::<Value>().await?;

    Ok(Record {
        country: city.country.to_owned(),
        name: city.name,
        description: res.get("extract").unwrap().to_string(),
        id: city.id,
    })
}

fn serialize_data(data: Vec<Record>) -> Result<(), Box<dyn std::error::Error>> {
    let f = File::open("./data.csv")?;
    let f = BufWriter::new(f);
    let mut wtr = csv::Writer::from_writer(f);

    for row in data {
        wtr.serialize(row)?;
    }

    wtr.flush()?;

    Ok(())
}

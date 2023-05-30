use reqwest::Client;
use reqwest::Method;

use serde::Serialize;
use serde_json::Value;

use csv;
use futures::{stream, StreamExt};
use std::error::Error;
use std::fs::OpenOptions;
use std::io::BufWriter;

const USER_AGENT: &str = "JustTestingForNow/0.0 (testing@protonmail.ch)";
const LENGTH_ID_PREFIX: usize = "http://www.wikidata.org/entity/".len();
const CONCURRENT_REQUESTS: usize = 50;

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
async fn main() -> Result<(), Box<dyn Error>> {
    compute_all("data.csv").await
}

async fn compute_all(output_file: &str) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(output_file)
        .unwrap();

    let countries = stream::iter(query_countries().await?)
        .map(|country| async {
            query_cities(country).await.unwrap()
        })
    .buffered(CONCURRENT_REQUESTS)
    .collect::<Vec<_>>()
    .await;
    for country in countries {
        let result = stream::iter(country.into_iter())
            .map(|city| async { get_record(city).await })
            .buffer_unordered(CONCURRENT_REQUESTS)
            .filter_map(|val| async { val.ok() })
            .collect::<Vec<_>>()
            .await;

        if let Err(e) = serialize_data(&mut file, result) {
            println!("Error printing! {}", e);
        }
    }
    Ok(())
}

async fn query_countries() -> Result<Vec<Country>, Box<dyn Error>> {
    // TODO: Rewrite by using schema:name like in cities query for name?
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

async fn query_cities(country: Country) -> Result<Vec<City>, Box<dyn Error>> {
    let query = format!(
        "
        SELECT DISTINCT ?entity ?article ?name WHERE {{
            ?article schema:about ?entity .
            ?article schema:isPartOf <https://en.wikipedia.org/> .
            ?article schema:name ?name .
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
                .get("name")
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
            country: country.name.clone(),
        })
        .collect();

    Ok(cities)
}

async fn get_record(city: City) -> Result<Record, Box<dyn Error>> {
    let query: &str = "https://en.wikipedia.org/w/api.php?";
    let mut attempts = 0;
    let res = loop {
        let request = Client::new()
            .request(Method::GET, query)
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .query(&[
                ("action", "query"),
                ("format", "json"),
                ("prop", "extracts"),
                ("exintro", "1"),
                ("explaintext", "1"),
                ("redirects", "1"),
                ("titles", &city.name),
            ]);

        let res = request.send().await.and_then(|req| req.error_for_status());

        match res {
            Ok(res) => break res,
            Err(err) if err.is_timeout() => {
                if 5 < attempts {
                    return Err(Box::new(err));
                }
                attempts += 1;
                println!(
                    "Request {} timed out {attempts} times. Trying again ...",
                    city.name
                );
                continue;
            }
            Err(err) => {
                println!("Error fetching {:?}: {:?}", city.name, err);
                return Err(Box::new(err));
            }
        }
    };

    let res = res.json::<Value>().await?;

    let val = res
        .get("query")
        .unwrap()
        .get("pages")
        .unwrap()
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();

    Ok(Record {
        country: city.country.to_owned(),
        name: val.get("title").unwrap().to_string(),
        description: val.get("extract").unwrap().to_string(),
        id: city.id,
    })
}

fn serialize_data<WR: std::io::Write>(f: &mut WR, data: Vec<Record>) -> Result<(), Box<dyn Error>> {
    let f = BufWriter::new(f);
    let mut wtr = csv::Writer::from_writer(f);

    for row in data {
        wtr.serialize(row)?;
    }

    wtr.flush()?;

    Ok(())
}

use std::collections::HashMap;

use reqwest::Client;
use reqwest::Method;

use serde::{Serialize, Deserialize};
use serde_json;

use std::fs;

const USER_AGENT: &str = "JustTestingForNow/0.0 (testing@protonmail.ch)";
const LENGTH_ID_PREFIX: usize = "http://www.wikidata.org/entity/".len();
const LENGTH_ARTICLE_PREFIX: usize = "https://en.wikipedia.org/wiki/".len();

#[derive(Serialize, Deserialize, Debug)]
struct Data {
    head: Head,
    results: Results,
}

#[derive(Serialize, Deserialize, Debug)]
struct Head {
    vars: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Results {
    bindings: Vec<Entry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Entry {
    // #[serde(alias = "country", alias = "city")]
    entity: EntityID,
    // #[serde(alias = "countryLabel", alias = "cityLabel")]
    // name: EntityLabel,
    article: Article,
}

#[derive(Serialize, Deserialize, Debug)]
struct EntityID {
    // #[serde(rename = "type")]
    // _type: String,
    value: String,
}

// #[derive(Serialize, Deserialize, Debug)]
// struct EntityLabel {
//     // #[serde(rename = "xml:lang")]
//     // xml_lang: String,
//     // #[serde(rename = "type")]
//     // _type: String,
//     value: String,
// }

#[derive(Serialize, Deserialize, Debug)]
struct Article {
    // #[serde(rename = "type")]
    // _type: String,
    value: String,
}

#[derive(Debug)]
struct Country {
    name: String,
    id: String,
}

impl From<Entry> for Country {
    fn from(entry: Entry) -> Self {
        Country {
            id: entry.entity.value[LENGTH_ID_PREFIX..].to_owned(),
            name: entry.article.value[LENGTH_ARTICLE_PREFIX..].to_owned(),
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
struct City {
    name: String,
    id: String,
}

impl From<Entry> for City {
    fn from(entry: Entry) -> Self {
        City {
            id: entry.entity.value[LENGTH_ID_PREFIX..].to_owned(),
            name: entry.article.value[LENGTH_ARTICLE_PREFIX..].to_owned(),
        }
    }
}




#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let countries: Vec<_> = query_countries().await?.collect();

    let d = query_cities(&countries[0]).await?;
    
    // let d = query_cities(
    //     Country {
    //         name: "Germany".to_owned(),
    //         id: "Q183".to_owned(),
    //         // article: "https://en.wikipedia.org/wiki/Germany".to_owned(),
    //     }
    // ).await?;

    let mut list: Vec<_> = d.collect();

    list.sort();

    for entry in list {
        println!("{:?}", entry);
    }

    // println!("{}", d);
    // let d = query_countries().await?;
    // println!("{:?}", d);
    // fs::write("./res.json", resp.clone())?;
    // let resp = fs::read_to_string("./res.json")?;

    // let p: Data = serde_json::from_str(&resp)?;

    // let results: Vec<_> = p
    //     .results
    //     .bindings
    //     .into_iter()
    //     .map(|val| (val.country_label.value, val.country.value, val.article.value))
    //     .collect();
    
    // println!("{results:#?}");

    
    // let resp = reqwest::get("https://httpbin.org/ip")
    //     .await?
    //     .json::<HashMap<String, String>>()
    //     .await?;
    // println!("{:#?}", p);
    Ok(())
}


async fn query_countries() -> Result<Box<dyn Iterator<Item=Country>>, Box<dyn std::error::Error>> {
    // let query = "
    // SELECT DISTINCT ?country ?countryLabel ?article WHERE {
    //   ?country wdt:P31 wd:Q6256 . 
    //   ?article schema:about ?country .
    //   ?article schema:isPartOf <https://en.wikipedia.org/>.
    //   FILTER NOT EXISTS {?country wdt:P31 wd:Q3024240}
    //   FILTER NOT EXISTS {?country wdt:P31 wd:Q28171280}
    //   OPTIONAL { ?country wdt:P576 ?dissolved } .
    //   FILTER (!BOUND(?dissolved)) 
    //   SERVICE wikibase:label { bd:serviceParam wikibase:language \"en\" . }
    // }
    // ORDER BY ?countryLabel
    // ";
    let query = "
        SELECT DISTINCT ?entity ?article WHERE {
          ?entity wdt:P31 wd:Q6256 . 
          ?article schema:about ?entity .
          ?article schema:isPartOf <https://en.wikipedia.org/>.
          FILTER NOT EXISTS {?entity wdt:P31 wd:Q3024240}
          FILTER NOT EXISTS {?entity wdt:P31 wd:Q28171280}
          OPTIONAL { ?entity wdt:P576 ?dissolved } .
          FILTER (!BOUND(?dissolved)) 
        }
        ORDER BY ?entityLabel
    ";
    Ok(
        Box::new(
            Client::new()
            .request(Method::GET, "https://query.wikidata.org/sparql?")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .query(&[
                ("query", query),
                ("format", "json"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<Data>()
            .await?
            .results
            .bindings
            .into_iter()
            .map(|entry| entry.into())
        )
    )
}

async fn query_cities(country: &Country) -> Result<Box<dyn Iterator<Item=City>>, Box<dyn std::error::Error>> {
    let query = format!("
    SELECT DISTINCT ?entity ?article WHERE {{
      ?article schema:about ?entity .
      ?article schema:isPartOf <https://en.wikipedia.org/>.
      ?entity (wdt:P31/(wdt:P279*)) wd:Q515.
      ?entity wdt:P17 wd:{}.
    }}
    ", country.id);
    println!("{query}");
    Ok(
        Box::new(
            Client::new()
            .request(Method::GET, "https://query.wikidata.org/sparql?")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .query(&[
                ("query", query),
                ("format", "json".to_owned()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<Data>()
            .await?
            .results
            .bindings
            .into_iter()
            .map(|entry| entry.into())
        )
    )
}


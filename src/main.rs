use std::collections::HashMap;

use reqwest::Client;
use reqwest::Method;

use serde::{Serialize, Deserialize};
use serde_json::Value;

use std::fs;

const USER_AGENT: &str = "JustTestingForNow/0.0 (testing@protonmail.ch)";
const LENGTH_ID_PREFIX: usize = "http://www.wikidata.org/entity/".len();
const LENGTH_ARTICLE_PREFIX: usize = "https://en.wikipedia.org/wiki/".len();

#[derive(Serialize, Deserialize, Debug)]
struct WikidataResponse {
    head: WikidataHead,
    results: WikidataResults,
}

#[derive(Serialize, Deserialize, Debug)]
struct WikidataHead {
    vars: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WikidataResults {
    bindings: Vec<WikidataEntryCountry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WikidataEntryCountry {
    // #[serde(alias = "country", alias = "city")]
    entity: EntityID,
    #[serde(rename = "entityLabel")]
    title: EntityLabel,
    article: Article,
}

#[derive(Serialize, Deserialize, Debug)]
struct WikidataEntryCity {
    // #[serde(alias = "country", alias = "city")]
    entity: EntityID,
    // #[serde(rename = "entityLabel")]
    // title: EntityLabel,
    article: Article,
}

#[derive(Serialize, Deserialize, Debug)]
struct EntityID {
    // #[serde(rename = "type")]
    // _type: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EntityLabel {
    // #[serde(rename = "xml:lang")]
    // xml_lang: String,
    // #[serde(rename = "type")]
    // _type: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Article {
    // #[serde(rename = "type")]
    // _type: String,
    value: String,
}

#[derive(Debug, Clone)]
struct Country {
    name: String,
    id: String,
}

// impl From<WikidataEntryCountry> for Country {
//     fn from(entry: WikidataEntryCountry) -> Self {
//         Country {
//             id: entry.entity.value[LENGTH_ID_PREFIX..].to_owned(),
//             name: entry.article.value[LENGTH_ARTICLE_PREFIX..].to_owned(),
//         }
//     }
// }

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct City {
    country: String, // Human-readable country name
    name: String, // Human-readable name
    // title: String, // https://en.wikipedia.org/wiki/<title>
    id: String, // Wikidata ID, always Q<some number>
}

// impl From<Entry> for City {
//     fn from(entry: Entry) -> Self {
//         City {
//             id: entry.entity.value[LENGTH_ID_PREFIX..].to_owned(),
//             name: entry.article.value[LENGTH_ARTICLE_PREFIX..].to_owned(),
//         }
//     }
// }



#[derive(Deserialize, Debug)]
struct SummaryResponse {
    extract: String,
}


#[derive(Serialize, Debug)]
struct Record {
    country: String, // Human-readable country name
    name: String, // Human-readable name
    description: String, // Summary paragraph from wikipedia
    id: String, // Wikidata ID, always Q<some number>
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let res = query_countries().await?;

    let cities = query_cities(res[0].clone()).await?;

    println!("{:?}", cities);

    // let countries: Vec<_> = query_countries().await?.collect();

    // let d = query_cities(&countries[0]).await?;
    
    // let mut list: Vec<_> = d.collect();

    // list.sort();

    // for entry in list {
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
    .query(&[
        ("query", query),
        ("format", "json"),
    ])
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
        .map(|val|
            Country {
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
                    .unwrap()
                    [LENGTH_ID_PREFIX..]
                    .to_owned(),
            }
        )
        .collect();

    Ok(countries)
}

async fn query_cities(country: Country) -> Result<Vec<City>, Box<dyn std::error::Error>> {
    let query = format!("
    SELECT DISTINCT ?entity ?article WHERE {{
      ?article schema:about ?entity .
      ?article schema:isPartOf <https://en.wikipedia.org/>.
      ?entity (wdt:P31/(wdt:P279*)) wd:Q515.
      ?entity wdt:P17 wd:{}.
    }}
    ", country.id);

    let res = Client::new()
    .request(Method::GET, "https://query.wikidata.org/sparql?")
    .header(reqwest::header::USER_AGENT, USER_AGENT)
    .query(&[
        ("query", query),
        ("format", "json".to_owned()),
    ])
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
        .map(|val|
            City {
                name: val
                    .get("article")
                    .unwrap()
                    .get("value")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    [LENGTH_ARTICLE_PREFIX..]
                    .to_owned(),
                id: val
                    .get("entity")
                    .unwrap()
                    .get("value")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    [LENGTH_ID_PREFIX..]
                    .to_owned(),
                country: country.name.clone(),
            }
        )
        .collect();

    Ok(cities)
}

// async fn query_countries() -> Result<Box<dyn Iterator<Item=Country>>, Box<dyn std::error::Error>> {
//     // let query = "
//     // SELECT DISTINCT ?country ?countryLabel ?article WHERE {
//     //   ?country wdt:P31 wd:Q6256 . 
//     //   ?article schema:about ?country .
//     //   ?article schema:isPartOf <https://en.wikipedia.org/>.
//     //   FILTER NOT EXISTS {?country wdt:P31 wd:Q3024240}
//     //   FILTER NOT EXISTS {?country wdt:P31 wd:Q28171280}
//     //   OPTIONAL { ?country wdt:P576 ?dissolved } .
//     //   FILTER (!BOUND(?dissolved)) 
//     //   SERVICE wikibase:label { bd:serviceParam wikibase:language \"en\" . }
//     // }
//     // ORDER BY ?countryLabel
//     // ";
//     let query = "
//         SELECT DISTINCT ?entity ?entityLabel ?article WHERE {
//           ?entity wdt:P31 wd:Q6256 . 
//           ?article schema:about ?entity .
//           ?article schema:isPartOf <https://en.wikipedia.org/>.
//           FILTER NOT EXISTS {?entity wdt:P31 wd:Q3024240}
//           FILTER NOT EXISTS {?entity wdt:P31 wd:Q28171280}
//           OPTIONAL { ?entity wdt:P576 ?dissolved } .
//           FILTER (!BOUND(?dissolved)) 
//           SERVICE wikibase:label { bd:serviceParam wikibase:language \"en\" . }
//         }
//         ORDER BY ?entityLabel
//     ";
//     Ok(
//         Box::new(
//             Client::new()
//             .request(Method::GET, "https://query.wikidata.org/sparql?")
//             .header(reqwest::header::USER_AGENT, USER_AGENT)
//             .query(&[
//                 ("query", query),
//                 ("format", "json"),
//             ])
//             .send()
//             .await?
//             .error_for_status()?
//             .json::<WikidataResponse>()
//             .await?
//             .results
//             .bindings
//             .into_iter()
//             .map(|entry| entry.into())
//         )
//     )
// }

// async fn query_cities<'a>(country: &'a Country) -> Result<Box<dyn Iterator<Item=City> + 'a>, Box<dyn std::error::Error>> {
//     let query = format!("
//     SELECT DISTINCT ?entity ?entityLabel ?article WHERE {{
//       ?article schema:about ?entity .
//       ?article schema:isPartOf <https://en.wikipedia.org/>.
//       ?entity (wdt:P31/(wdt:P279*)) wd:Q515.
//       ?entity wdt:P17 wd:{}.
//     }}
//     ", country.id);
//     println!("{query}");
//     Ok(
//         Box::new(
//             Client::new()
//             .request(Method::GET, "https://query.wikidata.org/sparql?")
//             .header(reqwest::header::USER_AGENT, USER_AGENT)
//             .query(&[
//                 ("query", query),
//                 ("format", "json".to_owned()),
//             ])
//             .send()
//             .await?
//             .error_for_status()?
//             .json::<WikidataResponse>()
//             .await?
//             .results
//             .bindings
//             .into_iter()
//             .map(|entry| 
//                     City {
//                         id: entry.entity.value[LENGTH_ID_PREFIX..].to_owned(),
//                         name: entry.article.value[LENGTH_ARTICLE_PREFIX..].to_owned(),
//                         country: &country.name,
//                         // country: "Hey".to_owned(),
//                         // title: entry.title.value,
//                     }
//                 )
//         )

//     )
// }

// async fn get_record(city: City<'_>) -> Result<Record, Box<dyn std::error::Error>> {
//     let query: String = "https://en.wikipedia.org/api/rest_v1/page/summary/".to_owned() + &city.name;

//     let res = loop {
//         let request = Client::new()
//                 .request(Method::GET, &query) 
//                 .header(reqwest::header::USER_AGENT, USER_AGENT)
//                 .send()
//                 .await?
//                 .error_for_status();

//         match request {
//             Ok(res) => { break res },
//             Err(err) => println!("Error fetching {:?}: {:?}", city.name, err),
//         }
//     };

//     let description = res.
//         json::<SummaryResponse>()
//         .await?
//         .extract;

//     Ok(
//         Record {
//             country: city.country.to_owned(),
//             name: city.name,
//             description,
//             id: city.id,
//         }
//     )
// }

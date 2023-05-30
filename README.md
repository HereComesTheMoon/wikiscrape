# Wikiscrape
Querying Wikidata, then aggregating data from various Wikipedia articles using public API endpoints. Concurrent, but not general purpose.

In its current form, obtains a list of all cities with existing English Wikipedia entries from Wikidata, and then queries each Wikipedia entry to obtain a summary of the city.

The results are serialized into a CSV file.

While Rust is satisfying to use, Python is vastly more convenient when it comes to writing quick scripts which will only be ran once or twice. Serde is powerful, but not necessary until it comes to maintenance, or a changing API.


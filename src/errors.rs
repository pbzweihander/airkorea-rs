error_chain! {
    foreign_links {
        Request(::reqwest::Error);
        Hjson(::hjson2json::Error);
        Json(::serde_json::Error);
    }
    errors {
        ParsePage {
            description("Error while parsing page")
            display("Error while parsing page")
        }
    }
}

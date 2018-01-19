error_chain! {
    foreign_links {
        Json(::serde_json::Error);
        Hjson(::serde_hjson::Error);
    }
}

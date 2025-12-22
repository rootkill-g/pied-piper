use wit_bindgen::generate;

generate!({
    path: "wit",
    world: "component-host-demo",
});

use piedpiper::component_host_demo::{host, http, storage};

struct Component;

impl exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        let (status, body) = http::get("https://example.com");
        let (found, value) = storage::get("greeting");

        let cached_len = if found { value.len() } else { 0 };
        let now = host::now_millis();

        host::log(&format!(
            "component-host-demo status={} body_len={} cached_len={} now={}",
            status,
            body.len(),
            cached_len,
            now
        ));

        let response_body = format!(
            "http_status={} body_len={} cache_hit={} cache_len={} now={}",
            status,
            body.len(),
            found,
            cached_len,
            now
        );

        let response_json = format!(
            r#"{{"status":200,"body":"{}","content_type":"text/plain"}}"#,
            response_body
        );

        println!("{}", response_json);

        Ok(())
    }
}

export!(Component);

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlInputElement, Request, RequestInit, RequestMode, Response};
use search_core::SearchResult;

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let val = document.create_element("div")?;
    val.set_inner_html(r#"
        <input type="text" id="search-input" placeholder="Search...">
        <button id="search-button">Search</button>
        <div id="results"></div>
    "#);
    body.append_child(&val)?;

    let input = document
        .get_element_by_id("search-input")
        .expect("should have search-input")
        .dyn_into::<HtmlInputElement>()?;

    let results_div = document
        .get_element_by_id("results")
        .expect("should have results div");

    let closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        let query = input.value();
        let results_div_inner = results_div.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match perform_search(&query).await {
                Ok(results) => {
                    let mut html = String::from("<ul>");
                    for res in results {
                        html.push_str(&format!("<li><strong>{}</strong>: {}</li>", res.title, res.description));
                    }
                    html.push_str("</ul>");
                    results_div_inner.set_inner_html(&html);
                }
                Err(_) => {
                    results_div_inner.set_inner_html("Error fetching results");
                }
            }
        });
    });

    document
        .get_element_by_id("search-button")
        .expect("should have search-button")
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;

    closure.forget();

    Ok(())
}

async fn perform_search(query: &str) -> Result<Vec<SearchResult>, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let url = format!("/api/search?q={}", js_sys::encode_uri_component(query));

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let resp: Response = resp_value.dyn_into().unwrap();

    let json = JsFuture::from(resp.json()?).await?;
    let results: Vec<SearchResult> = serde_wasm_bindgen::from_value(json)?;

    Ok(results)
}

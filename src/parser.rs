use scraper::{element_ref::Text, Html, Selector};
use url::Url;

use crate::Result;

fn collect_text(texts: Text) -> String {
    let mut result = String::with_capacity(2048);

    for text in texts {
        // not collect annotations math in Wikipedia
        if text.contains("\\displaystyle") || text.trim().is_empty() {
            continue;
        }

        result.push_str(text.trim());
        result.push_str(" ");
    }

    result.trim().to_owned()
}

pub async fn parse_document(url: String, body: String) -> Result<(String, String, Vec<String>)> {
    let document = Html::parse_document(&body);
    let url = Url::parse(&url)?;

    let content_selector = if url.domain().unwrap_or_default().contains("wikipedia.org") {
        Selector::parse("#content .mw-parser-output")?
    } else {
        Selector::parse("body")?
    };

    let title_selector = Selector::parse("title")?;
    let text_selector = Selector::parse("p, p ~ ul, p ~ ol")?;

    let title = match document.select(&title_selector).next() {
        Some(element) => element.text().collect::<Vec<_>>().join(""),
        _ => "".to_owned(),
    };

    let content_elements = document
        .select(&content_selector)
        .next()
        .ok_or("No content")?
        .select(&text_selector);

    let mut contents: Vec<String> = vec![];
    for element in content_elements {
        // add index in ol lists
        if element.value().name() == "ol" {
            let item_selector = Selector::parse("li")?;
            let mut list = vec![];

            for (index, item) in element.select(&item_selector).enumerate() {
                let text = collect_text(item.text());

                list.push(format!("{}. {}", index + 1, text))
            }

            contents.push(list.join("\n"));

            continue;
        }

        let text = collect_text(element.text());

        if text.is_empty() {
            continue;
        };

        contents.push(text.to_owned())
    }

    let description = contents
        .iter()
        .take(3)
        .map(|text| text.clone())
        .collect::<Vec<String>>()
        .join(" ");

    Ok((title, description, contents))
}

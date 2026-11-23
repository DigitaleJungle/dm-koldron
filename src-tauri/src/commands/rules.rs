//! Library → Rules: the contents (chapters and their sections from the switched-on packs) and one chapter as HTML.

use crate::commands::packs::{disabled_in, enabled, loaded};
use crate::packs::Pack;
use crate::storage::{blocking, StorageError};
use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use serde_json::{Map, Value};
use tauri::AppHandle;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContentsChapter {
    pub pack: String,
    pub pack_name: String,
    pub index: String,
    pub name: String,
    pub sections: Vec<ContentsSection>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ContentsSection {
    pub index: String,
    pub name: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Chapter {
    pub name: String,
    /// The chapter's intro, as HTML.
    pub html: String,
    /// Its picture (`image`, kept for its pack), if it has one: a homebrew chapter's.
    pub image: Option<String>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Section {
    pub index: String,
    pub name: String,
    pub html: String,
    /// Its picture, as for a chapter.
    pub image: Option<String>,
}

fn image(entry: &Map<String, Value>) -> Option<String> {
    entry.get("image").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
}

fn field<'a>(entry: &'a Map<String, Value>, name: &str) -> &'a str {
    entry.get(name).and_then(Value::as_str).unwrap_or("")
}

fn chapters(pack: &Pack) -> &[Map<String, Value>] {
    pack.entries.get("rules").map_or(&[], Vec::as_slice)
}

/// The rule sections a chapter lists, in its order. The loader checked that each exists.
fn sections_of<'a>(pack: &'a Pack, chapter: &Map<String, Value>) -> Vec<&'a Map<String, Value>> {
    let all = pack.entries.get("rule-sections").map_or(&[][..], Vec::as_slice);
    let refs = chapter.get("subsections").and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    refs.iter()
        .filter_map(|r| r.get("index").and_then(Value::as_str))
        .filter_map(|index| all.iter().find(|s| field(s, "index") == index))
        .collect()
}

pub fn contents(packs: &[&Pack]) -> Vec<ContentsChapter> {
    packs
        .iter()
        .flat_map(|pack| {
            chapters(pack).iter().map(|chapter| ContentsChapter {
                pack: pack.manifest.id.clone(),
                pack_name: pack.manifest.name.clone(),
                index: field(chapter, "index").into(),
                name: field(chapter, "name").into(),
                sections: sections_of(pack, chapter)
                    .into_iter()
                    .map(|s| ContentsSection { index: field(s, "index").into(), name: field(s, "name").into() })
                    .collect(),
            })
        })
        .collect()
}

pub fn chapter(packs: &[&Pack], pack: &str, index: &str) -> Result<Chapter, StorageError> {
    let not_found = || StorageError::NotFound(format!("No chapter \"{index}\" in the switched-on packs."));
    let pack = packs.iter().find(|p| p.manifest.id == pack).ok_or_else(not_found)?;
    let found = chapters(pack).iter().find(|c| field(c, "index") == index).ok_or_else(not_found)?;
    let name = field(found, "name");
    Ok(Chapter {
        name: name.into(),
        html: render(field(found, "desc"), name),
        image: image(found),
        sections: sections_of(pack, found)
            .into_iter()
            .map(|s| Section {
                index: field(s, "index").into(),
                name: field(s, "name").into(),
                html: render(field(s, "desc"), field(s, "name")),
                image: image(s),
            })
            .collect(),
    })
}

/// One rule section as HTML (the search screen's preview).
pub fn section(packs: &[&Pack], pack: &str, index: &str) -> Result<Section, StorageError> {
    let not_found = || StorageError::NotFound(format!("No rule section \"{index}\" in the switched-on packs."));
    let pack = packs.iter().find(|p| p.manifest.id == pack).ok_or_else(not_found)?;
    let all = pack.entries.get("rule-sections").map_or(&[][..], Vec::as_slice);
    let s = all.iter().find(|s| field(s, "index") == index).ok_or_else(not_found)?;
    Ok(Section { index: index.into(), name: field(s, "name").into(), html: render(field(s, "desc"), field(s, "name")), image: image(s) })
}

/// A tag in a pack's text: a link to `koldron:<kind>/<pack>/<index>` (each part letters, digits, `_`, `.` or `-`).
pub const TAG_SCHEME: &str = "koldron:";

/// A tag link's kind, pack and index, or none when `url` isn't a valid tag.
pub fn tag_parts(url: &str) -> Option<[&str; 3]> {
    let safe = |p: &str| (1..=100).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b));
    let mut parts = url.strip_prefix(TAG_SCHEME)?.split('/');
    let found = [parts.next()?, parts.next()?, parts.next()?];
    (parts.next().is_none() && found.iter().all(|p| safe(p))).then_some(found)
}

/// The text with each tag's address left out (`[Fireball](koldron:…)` → `[Fireball]`), for the search index.
pub fn without_tag_urls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("](koldron:") {
        let Some(end) = rest[at..].find(')') else { break };
        out.push_str(&rest[..=at]);
        rest = &rest[at + end + 1..];
    }
    out.push_str(rest);
    out
}

/// Markdown to HTML. Raw HTML in a pack's text is shown as text, and links and images are left out (keeping their
/// text), so nothing from a pack can run or load in the window; only a tag (`TAG_SCHEME`) stays a link, to
/// `/tag/…`, which `TagPreview.svelte` opens in the Library worked in. A leading heading that repeats `name` is dropped: the reader shows the
/// name itself.
pub fn render(markdown: &str, name: &str) -> String {
    let mut in_tag = false; // links don't nest, so one flag says whether a link's end closes a tag
    let parser = Parser::new_ext(without_title(markdown, name), Options::ENABLE_TABLES).filter_map(move |event| match event {
        Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text)),
        Event::Start(Tag::Link { dest_url, .. }) => {
            let [kind, pack, index] = tag_parts(&dest_url)?;
            in_tag = true;
            // The parts are checked to be safe characters only, so they need no escaping.
            Some(Event::InlineHtml(format!(r#"<a href="/tag/{kind}/{pack}/{index}" data-kind="{kind}" data-pack="{pack}" data-index="{index}">"#).into()))
        }
        Event::End(TagEnd::Link) if in_tag => {
            in_tag = false;
            Some(Event::InlineHtml("</a>".into()))
        }
        Event::Start(Tag::Image { .. } | Tag::HtmlBlock) => None,
        Event::End(TagEnd::Link | TagEnd::Image | TagEnd::HtmlBlock) => None,
        other => Some(other),
    });
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

fn without_title<'a>(markdown: &'a str, name: &str) -> &'a str {
    let text = markdown.trim_start();
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    let title = first.trim_start_matches('#');
    let is_heading = title.len() < first.len() && title.starts_with(' ');
    if is_heading && title.trim().eq_ignore_ascii_case(name.trim()) { rest } else { markdown }
}

#[tauri::command]
pub async fn rules_contents(app: AppHandle, campaign: Option<String>) -> Result<Vec<ContentsChapter>, StorageError> {
    blocking(move || Ok(contents(&enabled(&loaded(&app)?.0, &disabled_in(&app, campaign.as_deref())?)))).await
}

#[tauri::command]
pub async fn rules_chapter(app: AppHandle, pack: String, index: String, campaign: Option<String>) -> Result<Chapter, StorageError> {
    blocking(move || chapter(&enabled(&loaded(&app)?.0, &disabled_in(&app, campaign.as_deref())?), &pack, &index)).await
}

#[tauri::command]
pub async fn rules_section(app: AppHandle, pack: String, index: String, campaign: Option<String>) -> Result<Section, StorageError> {
    blocking(move || section(&enabled(&loaded(&app)?.0, &disabled_in(&app, campaign.as_deref())?), &pack, &index)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::load_dir;
    use std::path::Path;

    #[test]
    fn renders_markdown_without_raw_html_links_or_images() {
        let html = render("Hi <script>alert(1)</script> [run](javascript:alert(1)) ![x](http://e/x.png)\n\n<div onclick=\"x\">a</div>", "");
        assert!(!html.contains("<script") && !html.contains("<div") && !html.contains("href") && !html.contains("<img"), "{html}");
        assert!(html.contains("&lt;script&gt;") && html.contains("run"), "{html}");
        let table = render("| A | B |\n|---|---|\n| 1 | 2 |", "");
        assert!(table.contains("<table>") && table.contains("<td>2</td>"), "{table}");
        assert!(render("**bold** \\*not\\*", "").contains("<strong>bold</strong> *not*"));
    }

    #[test]
    fn drops_only_a_leading_heading_that_repeats_the_name() {
        assert_eq!(render("## D20 Tests\n\nText.", "D20 Tests"), "<p>Text.</p>\n");
    }

    #[test]
    fn a_tag_stays_a_link_to_the_entry_and_nothing_else_does() {
        let html = render("Cast [Fireball](koldron:spells/srd/fireball) or [run](https://e) [bad](koldron:spells/srd/a%22b) [x](koldron:a/b)", "");
        assert_eq!(
            html,
            "<p>Cast <a href=\"/tag/spells/srd/fireball\" data-kind=\"spells\" data-pack=\"srd\" data-index=\"fireball\">Fireball</a> or run bad x</p>\n"
        );
        assert_eq!(tag_parts("koldron:monsters/my-book/goblin.2"), Some(["monsters", "my-book", "goblin.2"]));
        assert_eq!(tag_parts("koldron:a/b/c/d"), None);
        assert_eq!(tag_parts("koldron:a//c"), None);
        assert_eq!(without_tag_urls("See [Fireball](koldron:spells/srd/fireball), [x](https://e)."), "See [Fireball], [x](https://e).");
        assert_eq!(render("# Playing the Game", "Playing the Game"), "");
        assert!(render("## Other\n\nText.", "D20 Tests").contains("<h2>Other</h2>"));
        assert!(render("D20 Tests\n\nText.", "D20 Tests").contains("<p>D20 Tests</p>"));
        assert!(render("#hashtag", "hashtag").contains("#hashtag"));
    }

    #[test]
    fn the_srd_contents_and_a_chapter() {
        let (packs, _) = load_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../packs")).unwrap();
        let on: Vec<_> = packs.iter().collect();
        let toc = contents(&on);
        assert_eq!(toc.len(), 7);
        assert_eq!((toc[0].pack.as_str(), toc[0].name.as_str()), ("srd", "Playing the Game"));
        assert_eq!(toc.iter().map(|c| c.sections.len()).sum::<usize>(), 192);
        let glossary = chapter(&on, "srd", "rules-glossary").unwrap();
        assert_eq!(glossary.sections.len(), 157);
        assert!(!glossary.sections[0].html.is_empty());
        assert!(!glossary.sections.iter().any(|s| s.html.starts_with("<h2>")), "section titles are dropped");
        assert!(matches!(chapter(&on, "srd", "nope"), Err(StorageError::NotFound(_))));
        assert!(matches!(chapter(&[], "srd", "rules-glossary"), Err(StorageError::NotFound(_))));
        let blinded = &glossary.sections.iter().find(|s| s.index == "blinded-condition").unwrap().html;
        assert_eq!(&section(&on, "srd", "blinded-condition").unwrap().html, blinded);
        assert!(matches!(section(&on, "srd", "nope"), Err(StorageError::NotFound(_))));
    }
}

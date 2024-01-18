//! ## A library to generate sitemaps.
//!
//! [![Version](https://img.shields.io/crates/v/sitewriter)](https://crates.io/crates/sitewriter)
//! [![Downloads](https://img.shields.io/crates/d/sitewriter)](https://crates.io/crates/sitewriter)
//! [![License](https://img.shields.io/crates/l/sitewriter)](https://crates.io/crates/sitewriter)
//! ![Rust](https://github.com/edg-l/sitewriter/workflows/Rust/badge.svg)
//! [![Docs](https://docs.rs/sitewriter/badge.svg)](https://docs.rs/sitewriter)
//! [![crev reviews](https://web.crev.dev/rust-reviews/badge/crev_count/sitewriter.svg)](https://web.crev.dev/rust-reviews/crate/sitewriter/)
//!
//! It uses the [quick-xml](https://github.com/tafia/quick-xml) so it should be fast.
//!
//! ## Example
//!
//! ```rust
//! use chrono::prelude::*;
//! use sitewriter::{ChangeFreq, UrlEntry, UrlEntryBuilder};
//!
//! let urls = vec![
//!     UrlEntryBuilder::default()
//!         .loc("https://edgarluque.com/projects".parse().unwrap())
//!         .build()
//!         .unwrap(),
//!     UrlEntry {
//!         loc: "https://edgarluque.com/".parse().unwrap(),
//!         changefreq: Some(ChangeFreq::Daily),
//!         priority: Some(1.0),
//!         lastmod: Some(Utc::now()),
//!     },
//!     UrlEntry {
//!         loc: "https://edgarluque.com/blog".parse().unwrap(),
//!         changefreq: Some(ChangeFreq::Weekly),
//!         priority: Some(0.8),
//!         lastmod: Some(Utc::now()),
//!     },
//!     UrlEntry {
//!         loc: "https://edgarluque.com/blog/sitewriter".parse().unwrap(),
//!         changefreq: Some(ChangeFreq::Never),
//!         priority: Some(0.5),
//!         lastmod: Some(Utc.ymd(2020, 11, 22).and_hms(15, 10, 15)),
//!     },
//!     UrlEntry {
//!         loc: "https://edgarluque.com/blog/some-future-post"
//!             .parse()
//!             .unwrap(),
//!         changefreq: Some(ChangeFreq::Never),
//!         priority: Some(0.5),
//!         lastmod: Some(
//!             Utc.from_utc_datetime(&Local.ymd(2020, 12, 5).and_hms(12, 30, 0).naive_utc()),
//!         ),
//!     },
//!     // Entity escaping
//!     UrlEntry {
//!         loc: "https://edgarluque.com/blog/test&id='<test>'"
//!             .parse()
//!             .unwrap(),
//!         changefreq: Some(ChangeFreq::Never),
//!         priority: Some(0.5),
//!         lastmod: Some(
//!             Utc.from_utc_datetime(&Local.ymd(2020, 12, 5).and_hms(12, 30, 0).naive_utc()),
//!         ),
//!     },
//! ];
//!
//! let result = sitewriter::generate_str(&urls);
//! println!("{}", result);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

use chrono::{DateTime, SecondsFormat, Utc};
use derive_builder::Builder;
use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use std::{fmt::Display, io::Cursor};

pub use quick_xml::Result;
pub use url::Url;

/// How frequently the page is likely to change. This value provides general
/// information to search engines and may not correlate exactly to how often they crawl the page.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChangeFreq {
    /// Changes each time it's accessed.
    Always,
    /// Changes hourly.
    Hourly,
    /// Changes daily.
    Daily,
    /// Changes weekly.
    Weekly,
    /// Changes monthly.
    Monthly,
    /// Changes yearly.
    Yearly,
    /// Describes archived URLs.
    Never,
}

impl Display for ChangeFreq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match self {
            Self::Always => "always",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
            Self::Never => "never",
        };
        f.write_str(what)
    }
}

/// A sitemap url entry.
#[derive(Debug, Clone, Builder, PartialEq, PartialOrd)]
#[builder(setter(strip_option))]
pub struct UrlEntry {
    /// URL of the page.
    ///
    /// This URL must begin with the protocol (such as http) and end with a trailing slash, if your web server requires it. This value must be less than 2,048 characters.
    pub loc: Url,
    /// The date of last modification of the file.
    #[builder(default)]
    pub lastmod: Option<DateTime<Utc>>,
    /// How frequently the page is likely to change.
    #[builder(default)]
    pub changefreq: Option<ChangeFreq>,
    /// The priority of this URL relative to other URLs on your site. Valid values range from 0.0 to 1.0.
    ///
    /// This value does not affect how your pages are compared to pages on other sites—it only lets the search engines know which pages you deem most important for the crawlers.
    #[builder(default)]
    pub priority: Option<f32>,
}

impl UrlEntry {
    /// Create a new url entry.
    #[must_use]
    pub const fn new(
        loc: Url,
        lastmod: Option<DateTime<Utc>>,
        changefreq: Option<ChangeFreq>,
        priority: Option<f32>,
    ) -> Self {
        Self {
            loc,
            lastmod,
            changefreq,
            priority,
        }
    }
}

fn write_tag<T>(writer: &mut Writer<T>, tag: &str, text: &str) -> Result<()>
where
    T: std::io::Write,
{
    writer.write_event(Event::Start(BytesStart::new(tag)))?;
    writer.write_event(Event::Text(BytesText::new(text)))?;
    writer.write_event(Event::End(BytesEnd::new(tag)))?;

    Ok(())
}

/// Generates the sitemap and saves it using the provided writer.
///
/// It's recommended to use [`generate_bytes`] or [`generate_str`] if you need a
/// String or a Vec<u8>.
///
/// # Errors
///
/// Will return `Err` if it fails to write to the writer.
pub fn generate<'g, T, I>(inner_writer: T, urls: I) -> Result<T>
where
    T: std::io::Write,
    I: IntoIterator<Item = &'g UrlEntry>,
{
    let mut writer = Writer::new_with_indent(inner_writer, b' ', 4);
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    let urlset_name = "urlset";
    let mut urlset = BytesStart::new(urlset_name);
    urlset.push_attribute(("xmlns", "http://www.sitemaps.org/schemas/sitemap/0.9"));
    writer.write_event(Event::Start(urlset))?;

    for entry in urls {
        writer.write_event(Event::Start(BytesStart::new("url")))?;

        write_tag(&mut writer, "loc", entry.loc.as_str())?;

        if let Some(lastmod) = &entry.lastmod {
            write_tag(
                &mut writer,
                "lastmod",
                &lastmod.to_rfc3339_opts(SecondsFormat::Secs, true),
            )?;
        }
        if let Some(priority) = &entry.priority {
            write_tag(&mut writer, "priority", &format!("{priority:.1}"))?;
        }
        if let Some(changefreq) = &entry.changefreq {
            write_tag(&mut writer, "changefreq", &changefreq.to_string())?;
        }

        writer.write_event(Event::End(BytesEnd::new("url")))?;
    }

    writer.write_event(Event::End(BytesEnd::new(urlset_name)))?;

    Ok(writer.into_inner())
}

/// Generates the sitemap.
#[must_use]
pub fn generate_bytes<'g, I>(urls: I) -> Vec<u8>
where
    I: IntoIterator<Item = &'g UrlEntry>,
{
    let inner = Cursor::new(Vec::new());
    let result = generate(inner, urls).expect(
            "it should never error, please report this bug to https://github.com/edg-l/sitewriter/issues",
        );
    result.into_inner()
}

/// Generates the sitemap returning a string.
#[must_use]
pub fn generate_str<'g, I>(urls: I) -> String
where
    I: IntoIterator<Item = &'g UrlEntry>,
{
    let bytes = generate_bytes(urls);
    let res = std::str::from_utf8(&bytes).expect("to be valid utf8");
    res.to_owned()
}

#[cfg(test)]
mod tests {
    use crate::{generate_str, ChangeFreq, UrlEntry, UrlEntryBuilder};

    #[test]
    fn it_works() {
        use chrono::Utc;

        let urls = vec![
            // Builder pattern
            UrlEntryBuilder::default()
                .loc("https://domain.com".parse().unwrap())
                .priority(0.2)
                .build()
                .unwrap(),
            // Using new
            UrlEntry::new(
                "https://domain.com/some_url".parse().unwrap(),
                None,
                None,
                None,
            ),
            // Initializing the struct.
            UrlEntry {
                loc: "https://domain.com/another".parse().unwrap(),
                priority: None,
                changefreq: Some(ChangeFreq::Always),
                lastmod: None,
            },
            UrlEntry {
                loc: "https://domain.com/url".parse().unwrap(),
                changefreq: Some(ChangeFreq::Daily),
                priority: Some(0.8),
                lastmod: Some(Utc::now()),
            },
            UrlEntry {
                loc: "https://domain.com/aa".parse().unwrap(),
                changefreq: Some(ChangeFreq::Monthly),
                priority: None,
                lastmod: None,
            },
            UrlEntry {
                loc: "https://domain.com/bb".parse().unwrap(),
                changefreq: None,
                priority: None,
                lastmod: None,
            },
            UrlEntry {
                loc: "https://domain.com/bb&id='<test>'".parse().unwrap(),
                changefreq: None,
                priority: Some(0.4),
                lastmod: None,
            },
        ];

        let _result = generate_str(&urls);
    }

    #[test]
    fn changefreq_is_valid() {
        assert_eq!(format!("{}", ChangeFreq::Always), "always");
        assert_eq!(format!("{}", ChangeFreq::Hourly), "hourly");
        assert_eq!(format!("{}", ChangeFreq::Daily), "daily");
        assert_eq!(format!("{}", ChangeFreq::Weekly), "weekly");
        assert_eq!(format!("{}", ChangeFreq::Monthly), "monthly");
        assert_eq!(format!("{}", ChangeFreq::Yearly), "yearly");
        assert_eq!(format!("{}", ChangeFreq::Never), "never");
    }
}

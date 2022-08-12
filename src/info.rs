use crate::refs::{ObjectReferences, RefType};
use pdf_writer::{Date as PDate, PdfWriter, TextStr};

#[derive(Default, Debug, Clone)]
pub struct Info {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
}

impl Info {
    pub fn new() -> Info {
        Info::default()
    }

    pub fn title<S: ToString>(&mut self, title: S) -> &mut Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn author<S: ToString>(&mut self, author: S) -> &mut Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn subject<S: ToString>(&mut self, subject: S) -> &mut Self {
        self.subject = Some(subject.to_string());
        self
    }

    pub fn keywords<S: ToString>(&mut self, keywords: S) -> &mut Self {
        self.keywords = Some(keywords.to_string());
        self
    }

    pub(crate) fn write(&self, refs: &mut ObjectReferences, writer: &mut PdfWriter) {
        let id = refs.gen(RefType::Info);
        let mut info = writer.document_info(id);

        if let Some(title) = &self.title {
            info.title(TextStr(title.as_str()));
        }
        if let Some(author) = &self.author {
            info.author(TextStr(author.as_str()));
        }
        if let Some(subject) = &self.subject {
            info.subject(TextStr(subject.as_str()));
        }
        if let Some(keywords) = &self.keywords {
            info.keywords(TextStr(keywords.as_str()));
        }
        info.creator(TextStr(concat!(
            env!("CARGO_PKG_NAME"),
            " v",
            env!("CARGO_PKG_VERSION")
        )));

        use chrono::prelude::*;
        let now = Local::now();
        let offset = now.offset().fix();
        let offset_hours = offset.local_minus_utc() / (60 * 60);
        let offset_minutes = ((offset.local_minus_utc() - (offset_hours * (60 * 60))) / 60).abs();
        let date = PDate::new(now.year() as u16)
            .month(now.month() as u8)
            .day(now.day() as u8)
            .hour(now.hour() as u8)
            .minute(now.minute() as u8)
            .second(now.second() as u8)
            .utc_offset_hour(offset_hours as i8)
            .utc_offset_minute(offset_minutes as u8);
        info.creation_date(date);
    }
}

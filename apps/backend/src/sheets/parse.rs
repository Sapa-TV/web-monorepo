use chrono::NaiveDate;

use crate::orders::game::{GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::movie::{MovieKind, NewMovieOrder};
use crate::orders::status::OrderStatus;
use crate::orders::vip::{NewVipRecord, VipKind};

fn cell(row: &[String], idx: usize) -> &str {
    row.get(idx).map(|s| s.trim()).unwrap_or("")
}

fn opt(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_data_row(row: &[String]) -> bool {
    cell(row, 1).parse::<u32>().is_ok()
}

fn parse_ru_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%d.%m.%Y").ok()
}

/// Accepts a raw spreadsheet id or any Google Sheets URL containing /d/{id}.
pub fn parse_spreadsheet_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if !trimmed.contains('/') {
        return Some(trimmed.to_string());
    }
    let mut parts = trimmed.split('/');
    while let Some(part) = parts.next() {
        if part == "d" {
            if let Some(id) = parts.next()
                && !id.is_empty()
            {
                return Some(id.to_string());
            }
            return None;
        }
    }
    None
}

/// Games tab: [_, №, title, customer, kind_text, completed_at, comment].
pub fn parse_game_rows(values: &[Vec<String>]) -> Vec<NewGameOrder> {
    values
        .iter()
        .filter(|row| is_data_row(row))
        .filter_map(|row| {
            let customer = cell(row, 3);
            if customer.is_empty() {
                return None;
            }
            let kind_text = cell(row, 4).to_lowercase();
            let kind = if kind_text.contains("прохожд") {
                GameOrderKind::Playthrough
            } else {
                GameOrderKind::Stream
            };
            let source = if kind_text.contains("рулетк") {
                OrderSource::Roulette
            } else if kind_text.contains("баллы") {
                OrderSource::Points
            } else if kind_text.contains("донат") {
                OrderSource::Donate
            } else {
                OrderSource::Other
            };
            let completed_at = parse_ru_date(cell(row, 5));
            let status = if completed_at.is_some() {
                OrderStatus::Completed
            } else {
                OrderStatus::Pending
            };
            Some(NewGameOrder::new(
                opt(cell(row, 2)),
                customer.to_string(),
                None,
                kind,
                source,
                status,
                completed_at,
                opt(cell(row, 6)),
            ))
        })
        .collect()
}

/// Movies tab: [_, №, _, customer, title, ...].
pub fn parse_movie_rows(values: &[Vec<String>]) -> Vec<NewMovieOrder> {
    values
        .iter()
        .filter(|row| is_data_row(row))
        .filter_map(|row| {
            let customer = cell(row, 3);
            if customer.is_empty() {
                return None;
            }
            Some(NewMovieOrder::new(
                opt(cell(row, 4)),
                customer.to_string(),
                None,
                MovieKind::Movie,
                OrderSource::Other,
                OrderStatus::Pending,
                None,
            ))
        })
        .collect()
}

/// VIP tab: [_, №, vip/unvip, customer, roulette_date, end_date].
pub fn parse_vip_rows(values: &[Vec<String>]) -> Vec<NewVipRecord> {
    values
        .iter()
        .filter(|row| is_data_row(row))
        .filter_map(|row| {
            let customer = cell(row, 3);
            if customer.is_empty() {
                return None;
            }
            let kind = if cell(row, 2).to_lowercase().contains("unvip") {
                VipKind::Unvip
            } else {
                VipKind::Vip
            };
            let roulette_date = parse_ru_date(cell(row, 4))?;
            Some(NewVipRecord::new(
                customer.to_string(),
                None,
                kind,
                roulette_date,
                parse_ru_date(cell(row, 5)),
                None,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn spreadsheet_id_from_variants() {
        assert_eq!(
            parse_spreadsheet_id("1Tv9aAfZTM1oGV9Pk-ANSJz--UiR_IObe1GlvkZhLPdk").as_deref(),
            Some("1Tv9aAfZTM1oGV9Pk-ANSJz--UiR_IObe1GlvkZhLPdk")
        );
        assert_eq!(
            parse_spreadsheet_id(
                "https://docs.google.com/spreadsheets/d/ABC123/edit?pli=1&gid=0#gid=0"
            )
            .as_deref(),
            Some("ABC123")
        );
        assert_eq!(
            parse_spreadsheet_id(" https://docs.google.com/spreadsheets/d/XYZ/ ").as_deref(),
            Some("XYZ")
        );
        assert_eq!(parse_spreadsheet_id(""), None);
        assert_eq!(parse_spreadsheet_id("https://google.com/"), None);
    }

    #[test]
    fn game_rows_skip_headers_and_parse_fields() {
        let values = vec![
            row(&["", "", "Пройденная", "Отмена", "", "", "легенда"]),
            row(&[
                "",
                "№",
                "ИГРА",
                "ЗАКАЗЧИК",
                "Стрим \\ прохождение",
                "Дата",
                "КОММЕНТАРИИ",
            ]),
            row(&[
                "",
                "1",
                "Subnautica",
                "Аноним",
                "Прохождение",
                "22.10.2023",
                "атмосферная",
            ]),
            row(&[
                "",
                "18",
                "Untitled goose game",
                "rondos767 (рулетка)",
                "Стрим",
                "18.01.2024",
                "",
            ]),
            row(&["", "99", "Мир танков", "СкуфБюджет", "30ч (донат)", "", ""]),
            row(&[
                "",
                "101",
                "SnowRunner",
                "Th0rN13",
                "стрим (баллы)",
                "---------",
                "",
            ]),
            row(&["", "102", "", "vzbzdnuvshiy", "прохождение (донат)", "", ""]),
            row(&["", "", "", "", "", "", ""]),
        ];

        let games = parse_game_rows(&values);
        assert_eq!(games.len(), 5);

        assert_eq!(games[0].title.as_deref(), Some("Subnautica"));
        assert_eq!(games[0].kind, GameOrderKind::Playthrough);
        assert_eq!(games[0].source, OrderSource::Other);
        assert_eq!(games[0].status, OrderStatus::Completed);
        assert_eq!(games[0].comment.as_deref(), Some("атмосферная"));

        assert_eq!(games[1].source, OrderSource::Other);
        assert_eq!(games[1].kind, GameOrderKind::Stream);

        assert_eq!(games[2].source, OrderSource::Donate);
        assert_eq!(games[2].status, OrderStatus::Pending);

        assert_eq!(games[3].source, OrderSource::Points);
        assert_eq!(games[3].status, OrderStatus::Pending);

        assert_eq!(games[4].title, None);
        assert_eq!(games[4].kind, GameOrderKind::Playthrough);
    }

    #[test]
    fn movie_rows_parse_customer_and_title() {
        let values = vec![
            row(&["", "№", "Фильм", "ЗАКАЗЧИК", "Фильм\\Сериал"]),
            row(&["", "341", "", "kor_win_", "Зеркальная маска"]),
            row(&["", "423", "", "nikolay_m_91 (Jeker3)", ""]),
            row(&["", "424", "", "", ""]),
        ];

        let movies = parse_movie_rows(&values);
        assert_eq!(movies.len(), 2);
        assert_eq!(movies[0].customer_name, "kor_win_");
        assert_eq!(movies[0].title.as_deref(), Some("Зеркальная маска"));
        assert_eq!(movies[0].kind, MovieKind::Movie);
        assert_eq!(movies[1].title, None);
    }

    #[test]
    fn vip_rows_parse_kind_and_dates() {
        let values = vec![
            row(&["", "447", "VIP", "kasperaas", "08.06.2026", "22.06.2026"]),
            row(&[
                "",
                "457",
                "UnVIP",
                "JackTheRizer",
                "17.07.2026",
                "24.07.2026",
            ]),
            row(&["", "480", "VIP", "", "13.01.1900", ""]),
            row(&["", "481", "VIP", "broken", "not-a-date", ""]),
        ];

        let vips = parse_vip_rows(&values);
        assert_eq!(vips.len(), 2);
        assert_eq!(vips[0].kind, VipKind::Vip);
        assert_eq!(
            vips[0].roulette_date,
            NaiveDate::from_ymd_opt(2026, 6, 8).unwrap()
        );
        assert_eq!(
            vips[0].end_date,
            Some(NaiveDate::from_ymd_opt(2026, 6, 22).unwrap())
        );
        assert_eq!(vips[1].kind, VipKind::Unvip);
    }
}
